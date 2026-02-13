use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::common::expand_source_path;
use crate::trace::function::{FunctionChange, FunctionOperation};

/// Maximum number of body lines to include in output
const MAX_BODY_PREVIEW_LINES: usize = 5;

// Static regex for source commands (compiled once)
static SOURCE_CMD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(\.|source)\s+(.+)$").unwrap());

/// A parsed entry from a shell script (function context)
#[derive(Debug, Clone)]
pub enum ParsedFunctionEntry {
    /// A function definition, autoload, or unset
    Definition(FunctionChange),
    /// A source/. command pointing to another file
    Source(PathBuf),
}

#[cfg(test)]
impl ParsedFunctionEntry {
    fn as_definition(&self) -> &FunctionChange {
        match self {
            ParsedFunctionEntry::Definition(c) => c,
            ParsedFunctionEntry::Source(_) => panic!("Expected Definition variant"),
        }
    }

    fn is_source(&self) -> bool {
        matches!(self, ParsedFunctionEntry::Source(_))
    }
}

/// Parse a shell script file for function definitions
pub fn parse_shell_file_for_function(
    path: &Path,
    target_func: &str,
) -> std::io::Result<Vec<ParsedFunctionEntry>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_shell_content_for_function(
        &content,
        path,
        target_func,
    ))
}

/// Parse shell content for function definitions (testable without filesystem)
fn parse_shell_content_for_function(
    content: &str,
    path: &Path,
    target_func: &str,
) -> Vec<ParsedFunctionEntry> {
    let mut results = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    // Dynamic regex patterns (depend on target function name)
    let escaped = regex::escape(target_func);
    let posix_func = Regex::new(&format!(r"^\s*{}\s*\(\)\s*\{{", escaped)).unwrap();
    let bash_func = Regex::new(&format!(r"^\s*function\s+{}\s*\{{", escaped)).unwrap();
    let hybrid_func = Regex::new(&format!(r"^\s*function\s+{}\s*\(\)\s*\{{", escaped)).unwrap();
    let autoload = Regex::new(&format!(
        r"^\s*autoload\s+(?:-[A-Za-z]+\s+)*{}(?:\s|$)",
        escaped
    ))
    .unwrap();
    let unset_func = Regex::new(&format!(r"^\s*unset\s+-f\s+{}(?:\s|$)", escaped)).unwrap();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            i += 1;
            continue;
        }

        // Check for source/. command
        if let Some(caps) = SOURCE_CMD.captures(line) {
            let source_path = caps.get(2).unwrap().as_str().trim();
            if let Some(p) = expand_source_path(source_path) {
                results.push(ParsedFunctionEntry::Source(p));
            }
            i += 1;
            continue;
        }

        // Check for unset -f
        if unset_func.is_match(line) {
            results.push(ParsedFunctionEntry::Definition(FunctionChange {
                file: path.to_path_buf(),
                line_number: i + 1,
                line_content: line.to_string(),
                operation: FunctionOperation::Unset,
                body: None,
                body_lines: 0,
            }));
            i += 1;
            continue;
        }

        // Check for autoload
        if autoload.is_match(line) {
            results.push(ParsedFunctionEntry::Definition(FunctionChange {
                file: path.to_path_buf(),
                line_number: i + 1,
                line_content: line.to_string(),
                operation: FunctionOperation::Autoload,
                body: None,
                body_lines: 0,
            }));
            i += 1;
            continue;
        }

        // Check for function definitions (hybrid first, then bash, then POSIX)
        if hybrid_func.is_match(line) || bash_func.is_match(line) || posix_func.is_match(line) {
            let (body, body_lines) = extract_function_body(&lines, i);
            results.push(ParsedFunctionEntry::Definition(FunctionChange {
                file: path.to_path_buf(),
                line_number: i + 1,
                line_content: line.to_string(),
                operation: FunctionOperation::Define,
                body: Some(body),
                body_lines,
            }));
            // Skip past the function body
            i += body_lines.max(1);
            continue;
        }

        i += 1;
    }

    results
}

/// Extract the function body by tracking brace depth
///
/// Returns (body_preview, total_lines) where body_preview contains
/// at most MAX_BODY_PREVIEW_LINES of the body content.
fn extract_function_body(lines: &[&str], start: usize) -> (String, usize) {
    let mut depth: i32 = 0;
    let mut body_lines: Vec<&str> = Vec::new();
    let mut total_lines: usize = 0;

    for (offset, &line) in lines[start..].iter().enumerate() {
        // Count braces (simple approach - doesn't handle braces in strings/comments)
        for ch in line.chars() {
            match ch {
                '{' => depth += 1,
                '}' => depth -= 1,
                _ => {}
            }
        }

        total_lines = offset + 1;

        // Collect body lines (skip the opening line with the function name)
        if offset > 0 {
            body_lines.push(line);
        }

        // Closing brace at depth 0 means function is complete
        if depth == 0 && offset > 0 {
            break;
        }
    }

    // Build preview: first MAX_BODY_PREVIEW_LINES of the body
    let preview = if body_lines.len() <= MAX_BODY_PREVIEW_LINES {
        body_lines.join("\n")
    } else {
        let mut preview = body_lines[..MAX_BODY_PREVIEW_LINES].join("\n");
        preview.push_str("\n...");
        preview
    };

    (preview, total_lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_posix_function() {
        let content = "my_func() {\n    echo hello\n}\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].as_definition().operation,
            FunctionOperation::Define
        );
        assert_eq!(results[0].as_definition().line_number, 1);
        assert!(results[0].as_definition().body.is_some());
        assert_eq!(results[0].as_definition().body_lines, 3);
    }

    #[test]
    fn test_bash_function() {
        let content = "function my_func {\n    echo hello\n}\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].as_definition().operation,
            FunctionOperation::Define
        );
    }

    #[test]
    fn test_hybrid_function() {
        let content = "function my_func() {\n    echo hello\n}\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].as_definition().operation,
            FunctionOperation::Define
        );
    }

    #[test]
    fn test_multiline_body_with_truncation() {
        let content = "my_func() {\n    line1\n    line2\n    line3\n    line4\n    line5\n    line6\n    line7\n}\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        let body = results[0].as_definition().body.as_ref().unwrap();
        assert!(body.contains("..."));
        assert_eq!(results[0].as_definition().body_lines, 9);
    }

    #[test]
    fn test_one_liner_function() {
        let content = "my_func() { echo hello; }\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_definition().body_lines, 1);
    }

    #[test]
    fn test_nested_braces() {
        let content = "my_func() {\n    if true; then\n        echo yes\n    fi\n    for i in 1 2; do\n        echo $i\n    done\n}\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_definition().body_lines, 8);
    }

    #[test]
    fn test_nested_braces_in_body() {
        let content = "my_func() {\n    if [ -z \"$x\" ]; then\n        {\n            echo nested\n        }\n    fi\n}\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].as_definition().body_lines, 7);
    }

    #[test]
    fn test_autoload_basic() {
        let content = "autoload my_func\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].as_definition().operation,
            FunctionOperation::Autoload
        );
        assert!(results[0].as_definition().body.is_none());
        assert_eq!(results[0].as_definition().body_lines, 0);
    }

    #[test]
    fn test_autoload_with_flags() {
        let content = "autoload -Uz my_func\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].as_definition().operation,
            FunctionOperation::Autoload
        );
    }

    #[test]
    fn test_autoload_with_u_flag() {
        let content = "autoload -U my_func\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].as_definition().operation,
            FunctionOperation::Autoload
        );
    }

    #[test]
    fn test_unset_function() {
        let content = "unset -f my_func\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].as_definition().operation,
            FunctionOperation::Unset
        );
        assert!(results[0].as_definition().body.is_none());
    }

    #[test]
    fn test_comments_skipped() {
        let content = "# my_func() {\n# echo hello\n# }\nother_func() {\n    echo world\n}\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_other_function_skipped() {
        let content = "other_func() {\n    echo hello\n}\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_source_commands_returned() {
        let content = "source ~/.zshrc\n. /etc/profile\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 2);
        assert!(results[0].is_source());
        assert!(results[1].is_source());
    }

    #[test]
    fn test_multiple_definitions_same_file() {
        let content = "my_func() {\n    echo first\n}\nmy_func() {\n    echo second\n}\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0].as_definition().operation,
            FunctionOperation::Define
        );
        assert_eq!(
            results[1].as_definition().operation,
            FunctionOperation::Define
        );
    }

    #[test]
    fn test_body_content() {
        let content = "my_func() {\n    echo hello\n    echo world\n}\n";
        let results = parse_shell_content_for_function(content, &PathBuf::from("test"), "my_func");
        assert_eq!(results.len(), 1);
        let body = results[0].as_definition().body.as_ref().unwrap();
        assert!(body.contains("echo hello"));
        assert!(body.contains("echo world"));
    }
}
