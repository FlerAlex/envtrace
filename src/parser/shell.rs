use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::common::{expand_source_path, strip_quotes};
use crate::trace::{Operation, VariableChange};

// Static regex patterns (compiled once)
static EXPORT_ASSIGN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*export\s+([A-Za-z_][A-Za-z0-9_]*)=(.*)$"#).unwrap());
static SIMPLE_ASSIGN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*([A-Za-z_][A-Za-z0-9_]*)=(.*)$"#).unwrap());
static EXPORT_ONLY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*export\s+([A-Za-z_][A-Za-z0-9_]*)\s*$"#).unwrap());
static UNSET_VAR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*unset\s+([A-Za-z_][A-Za-z0-9_]*)"#).unwrap());
static SOURCE_CMD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"^\s*(\.|source)\s+(.+)$"#).unwrap());
static CONDITIONAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"^\s*\[.*\]\s*&&\s*export\s+([A-Za-z_][A-Za-z0-9_]*)=(.*)$"#).unwrap()
});

/// A parsed entry from a shell script
#[derive(Debug, Clone)]
pub enum ParsedShellEntry {
    /// A variable assignment (export, set, append, prepend, unset, conditional)
    Assignment(VariableChange),
    /// A source/. command pointing to another file
    Source(PathBuf),
}

#[cfg(test)]
impl ParsedShellEntry {
    fn as_assignment(&self) -> &VariableChange {
        match self {
            ParsedShellEntry::Assignment(c) => c,
            ParsedShellEntry::Source(_) => panic!("Expected Assignment variant"),
        }
    }
}

/// Parse a shell script file for variable assignments
///
/// Handles common bash/zsh patterns:
/// - export VAR=value
/// - VAR=value
/// - export VAR (export existing variable)
/// - unset VAR
/// - PATH="$PATH:new" (append)
/// - PATH="new:$PATH" (prepend)
/// - [ -f x ] && export VAR=y (conditional)
/// - source file / . file
pub fn parse_shell_file(
    path: &Path,
    target_var: &str,
    current_value: Option<&str>,
) -> std::io::Result<Vec<ParsedShellEntry>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_shell_content(
        &content,
        path,
        target_var,
        current_value,
    ))
}

fn parse_shell_content(
    content: &str,
    path: &Path,
    target_var: &str,
    current_value: Option<&str>,
) -> Vec<ParsedShellEntry> {
    let mut results = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Check for source/. command
        if let Some(caps) = SOURCE_CMD.captures(line) {
            let source_path = caps.get(2).unwrap().as_str().trim();
            if let Some(p) = expand_source_path(source_path) {
                results.push(ParsedShellEntry::Source(p));
            }
            continue;
        }

        // Check for conditional export
        if let Some(caps) = CONDITIONAL.captures(line) {
            let var_name = caps.get(1).unwrap().as_str();
            if var_name == target_var {
                let value = caps.get(2).unwrap().as_str();
                let value = strip_quotes(value);
                results.push(ParsedShellEntry::Assignment(VariableChange {
                    file: path.to_path_buf(),
                    line_number: line_num + 1,
                    line_content: line.to_string(),
                    operation: Operation::Conditional,
                    value_before: current_value.map(|s| s.to_string()),
                    value_after: value,
                }));
            }
            continue;
        }

        // Check for unset
        if let Some(caps) = UNSET_VAR.captures(line) {
            let var_name = caps.get(1).unwrap().as_str();
            if var_name == target_var {
                results.push(ParsedShellEntry::Assignment(VariableChange {
                    file: path.to_path_buf(),
                    line_number: line_num + 1,
                    line_content: line.to_string(),
                    operation: Operation::Unset,
                    value_before: current_value.map(|s| s.to_string()),
                    value_after: String::new(),
                }));
            }
            continue;
        }

        // Check for export VAR=value
        if let Some(caps) = EXPORT_ASSIGN.captures(line) {
            let var_name = caps.get(1).unwrap().as_str();
            if var_name == target_var {
                let value = caps.get(2).unwrap().as_str();
                let (operation, final_value) =
                    analyze_value(target_var, value, current_value, Operation::Export);
                results.push(ParsedShellEntry::Assignment(VariableChange {
                    file: path.to_path_buf(),
                    line_number: line_num + 1,
                    line_content: line.to_string(),
                    operation,
                    value_before: current_value.map(|s| s.to_string()),
                    value_after: final_value,
                }));
            }
            continue;
        }

        // Check for export VAR (without assignment)
        if let Some(caps) = EXPORT_ONLY.captures(line) {
            let var_name = caps.get(1).unwrap().as_str();
            if var_name == target_var {
                // Just exporting, value doesn't change
                results.push(ParsedShellEntry::Assignment(VariableChange {
                    file: path.to_path_buf(),
                    line_number: line_num + 1,
                    line_content: line.to_string(),
                    operation: Operation::Export,
                    value_before: current_value.map(|s| s.to_string()),
                    value_after: current_value.unwrap_or("").to_string(),
                }));
            }
            continue;
        }

        // Check for simple VAR=value (without export)
        if let Some(caps) = SIMPLE_ASSIGN.captures(line) {
            let var_name = caps.get(1).unwrap().as_str();
            if var_name == target_var {
                let value = caps.get(2).unwrap().as_str();
                let (operation, final_value) =
                    analyze_value(target_var, value, current_value, Operation::Set);
                results.push(ParsedShellEntry::Assignment(VariableChange {
                    file: path.to_path_buf(),
                    line_number: line_num + 1,
                    line_content: line.to_string(),
                    operation,
                    value_before: current_value.map(|s| s.to_string()),
                    value_after: final_value,
                }));
            }
        }
    }

    results
}

/// Analyze a value to determine if it's a set, append, or prepend operation
fn analyze_value(
    var_name: &str,
    value: &str,
    current_value: Option<&str>,
    default_op: Operation,
) -> (Operation, String) {
    let value = strip_quotes(value);
    let var_ref = format!("${}", var_name);
    let var_ref_braced = format!("${{{}}}", var_name);

    // Check for append pattern: $VAR:new or ${VAR}:new
    if value.starts_with(&var_ref) || value.starts_with(&var_ref_braced) {
        let prefix_len = if value.starts_with(&var_ref_braced) {
            var_ref_braced.len()
        } else {
            var_ref.len()
        };

        if value.len() > prefix_len && value.chars().nth(prefix_len) == Some(':') {
            let appended = &value[prefix_len + 1..];
            let new_value = if let Some(cur) = current_value {
                format!("{}:{}", cur, appended)
            } else {
                appended.to_string()
            };
            return (Operation::Append, new_value);
        }
    }

    // Check for prepend pattern: new:$VAR or new:${VAR}
    if value.ends_with(&var_ref) || value.ends_with(&var_ref_braced) {
        let suffix_len = if value.ends_with(&var_ref_braced) {
            var_ref_braced.len()
        } else {
            var_ref.len()
        };

        let value_len = value.len();
        if value_len > suffix_len {
            let before_var = &value[..value_len - suffix_len];
            if let Some(prepended) = before_var.strip_suffix(':') {
                let new_value = if let Some(cur) = current_value {
                    format!("{}:{}", prepended, cur)
                } else {
                    prepended.to_string()
                };
                return (Operation::Prepend, new_value);
            }
        }
    }

    // Simple set - expand any variable references we know about
    let expanded = expand_variables(&value, var_name, current_value);
    (default_op, expanded)
}

/// Expand known variable references
fn expand_variables(value: &str, var_name: &str, current_value: Option<&str>) -> String {
    let mut result = value.to_string();

    // Expand $HOME
    if let Some(home) = dirs::home_dir() {
        result = result.replace("$HOME", &home.to_string_lossy());
        result = result.replace("${HOME}", &home.to_string_lossy());
    }

    // Expand self-reference if we have a current value
    if let Some(cur) = current_value {
        let var_ref = format!("${}", var_name);
        let var_ref_braced = format!("${{{}}}", var_name);
        result = result.replace(&var_ref_braced, cur);
        result = result.replace(&var_ref, cur);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_assignment() {
        let content = "export PATH=/usr/bin";
        let results = parse_shell_content(content, &PathBuf::from("test"), "PATH", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_assignment().value_after, "/usr/bin");
        assert_eq!(results[0].as_assignment().operation, Operation::Export);
    }

    #[test]
    fn test_simple_assignment() {
        let content = "EDITOR=vim";
        let results = parse_shell_content(content, &PathBuf::from("test"), "EDITOR", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_assignment().value_after, "vim");
        assert_eq!(results[0].as_assignment().operation, Operation::Set);
    }

    #[test]
    fn test_append() {
        let content = r#"export PATH="$PATH:/usr/local/bin""#;
        let results =
            parse_shell_content(content, &PathBuf::from("test"), "PATH", Some("/usr/bin"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_assignment().operation, Operation::Append);
        assert_eq!(
            results[0].as_assignment().value_after,
            "/usr/bin:/usr/local/bin"
        );
    }

    #[test]
    fn test_prepend() {
        let content = r#"export PATH="/usr/local/bin:$PATH""#;
        let results =
            parse_shell_content(content, &PathBuf::from("test"), "PATH", Some("/usr/bin"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_assignment().operation, Operation::Prepend);
        assert_eq!(
            results[0].as_assignment().value_after,
            "/usr/local/bin:/usr/bin"
        );
    }

    #[test]
    fn test_unset() {
        let content = "unset PATH";
        let results =
            parse_shell_content(content, &PathBuf::from("test"), "PATH", Some("/usr/bin"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_assignment().operation, Operation::Unset);
    }

    #[test]
    fn test_skip_comments() {
        let content = "# export PATH=/bad\nexport PATH=/good";
        let results = parse_shell_content(content, &PathBuf::from("test"), "PATH", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_assignment().value_after, "/good");
    }

    #[test]
    fn test_conditional() {
        let content = "[ -f /etc/profile ] && export PATH=/usr/bin";
        let results = parse_shell_content(content, &PathBuf::from("test"), "PATH", None);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_assignment().operation, Operation::Conditional);
    }

    #[test]
    fn test_braced_variable() {
        let content = r#"export PATH="${PATH}:/new""#;
        let results = parse_shell_content(content, &PathBuf::from("test"), "PATH", Some("/old"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_assignment().operation, Operation::Append);
        assert_eq!(results[0].as_assignment().value_after, "/old:/new");
    }
}
