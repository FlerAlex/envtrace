use std::path::Path;

use crate::trace::{Operation, VariableChange};

/// Parse an /etc/environment style file (simple KEY=value format)
///
/// This format is used by PAM on Linux. It does NOT support:
/// - Shell variable expansion ($VAR)
/// - Command substitution
/// - Comments on the same line as assignments
///
/// It DOES support:
/// - Quoted values (single or double)
/// - Comments (lines starting with #)
/// - Blank lines
pub fn parse_environment_file(
    path: &Path,
    target_var: &str,
) -> std::io::Result<Vec<VariableChange>> {
    let content = std::fs::read_to_string(path)?;
    Ok(parse_environment_content(&content, path, target_var))
}

fn parse_environment_content(content: &str, path: &Path, target_var: &str) -> Vec<VariableChange> {
    let mut changes = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Parse KEY=value
        if let Some((key, value)) = parse_assignment(line)
            && key == target_var
        {
            changes.push(VariableChange {
                file: path.to_path_buf(),
                line_number: line_num + 1, // 1-indexed
                line_content: line.to_string(),
                operation: Operation::Set,
                value_before: None,
                value_after: value,
            });
        }
    }

    changes
}

/// Parse a KEY=value assignment, handling quotes
fn parse_assignment(line: &str) -> Option<(String, String)> {
    let eq_pos = line.find('=')?;
    let key = line[..eq_pos].trim();
    let value_part = line[eq_pos + 1..].trim();

    // Validate key (must be a valid identifier)
    if !is_valid_identifier(key) {
        return None;
    }

    // Remove surrounding quotes if present
    let value = if (value_part.starts_with('"') && value_part.ends_with('"'))
        || (value_part.starts_with('\'') && value_part.ends_with('\''))
    {
        value_part[1..value_part.len() - 1].to_string()
    } else {
        value_part.to_string()
    };

    Some((key.to_string(), value))
}

fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_simple_assignment() {
        let content = "PATH=/usr/bin:/bin";
        let changes =
            parse_environment_content(content, &PathBuf::from("/etc/environment"), "PATH");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].value_after, "/usr/bin:/bin");
        assert_eq!(changes[0].operation, Operation::Set);
    }

    #[test]
    fn test_quoted_value() {
        let content = r#"PATH="/usr/local/bin:/usr/bin""#;
        let changes =
            parse_environment_content(content, &PathBuf::from("/etc/environment"), "PATH");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].value_after, "/usr/local/bin:/usr/bin");
    }

    #[test]
    fn test_single_quoted_value() {
        let content = "EDITOR='vim'";
        let changes =
            parse_environment_content(content, &PathBuf::from("/etc/environment"), "EDITOR");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].value_after, "vim");
    }

    #[test]
    fn test_skip_comments() {
        let content = "# This is a comment\nPATH=/usr/bin";
        let changes =
            parse_environment_content(content, &PathBuf::from("/etc/environment"), "PATH");
        assert_eq!(changes.len(), 1);
    }

    #[test]
    fn test_skip_blank_lines() {
        let content = "\n\nPATH=/usr/bin\n\n";
        let changes =
            parse_environment_content(content, &PathBuf::from("/etc/environment"), "PATH");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].line_number, 3);
    }

    #[test]
    fn test_other_variable() {
        let content = "PATH=/usr/bin\nEDITOR=vim";
        let changes =
            parse_environment_content(content, &PathBuf::from("/etc/environment"), "EDITOR");
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].value_after, "vim");
    }

    #[test]
    fn test_variable_not_found() {
        let content = "PATH=/usr/bin";
        let changes =
            parse_environment_content(content, &PathBuf::from("/etc/environment"), "EDITOR");
        assert!(changes.is_empty());
    }
}
