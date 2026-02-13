//! Parser for macOS launchd plist files
//!
//! launchd plist files can set environment variables in two ways:
//! 1. EnvironmentVariables dict within the plist
//! 2. ProgramArguments that run `launchctl setenv VAR value`

use std::path::Path;

use crate::trace::VariableChange;

/// Parse a launchd plist file for environment variable definitions
#[cfg(target_os = "macos")]
pub fn parse_plist_file(path: &Path, target_var: &str) -> std::io::Result<Vec<VariableChange>> {
    use crate::trace::Operation;
    use plist::Value;

    let value = plist::from_file(path)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    let mut changes = Vec::new();

    if let Value::Dictionary(dict) = value {
        // Check for EnvironmentVariables dict
        if let Some(Value::Dictionary(env_vars)) = dict.get("EnvironmentVariables")
            && let Some(Value::String(val)) = env_vars.get(target_var)
        {
            changes.push(VariableChange {
                file: path.to_path_buf(),
                line_number: 0, // plist doesn't have meaningful line numbers
                line_content: format!("EnvironmentVariables.{} = {}", target_var, val),
                operation: Operation::Set,
                value_before: None,
                value_after: val.clone(),
            });
        }

        // Check for ProgramArguments containing launchctl setenv
        if let Some(Value::Array(args)) = dict.get("ProgramArguments") {
            let args: Vec<&str> = args
                .iter()
                .filter_map(|v| {
                    if let Value::String(s) = v {
                        Some(s.as_str())
                    } else {
                        None
                    }
                })
                .collect();

            // Look for: launchctl setenv VAR value
            for window in args.windows(4) {
                let is_launchctl = window[0].ends_with("launchctl") || window[0] == "launchctl";
                let is_setenv = window[1] == "setenv";
                let matches_var = window[2] == target_var;

                if is_launchctl && is_setenv && matches_var {
                    changes.push(VariableChange {
                        file: path.to_path_buf(),
                        line_number: 0,
                        line_content: format!("launchctl setenv {} {}", target_var, window[3]),
                        operation: Operation::Set,
                        value_before: None,
                        value_after: window[3].to_string(),
                    });
                }
            }
        }
    }

    Ok(changes)
}

/// Stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub fn parse_plist_file(_path: &Path, _target_var: &str) -> std::io::Result<Vec<VariableChange>> {
    Ok(vec![])
}

/// Query launchctl for the current value of an environment variable
///
/// This shows what GUI apps will actually see.
#[cfg(target_os = "macos")]
pub fn launchctl_getenv(var: &str) -> Option<String> {
    use std::process::Command;

    let output = Command::new("launchctl")
        .args(["getenv", var])
        .output()
        .ok()?;

    if output.status.success() {
        let value = String::from_utf8_lossy(&output.stdout);
        let value = value.trim();
        if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        }
    } else {
        None
    }
}

/// Stub for non-macOS platforms
#[cfg(not(target_os = "macos"))]
pub fn launchctl_getenv(_var: &str) -> Option<String> {
    None
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_environment_variables() {
        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>test.environment</string>
    <key>EnvironmentVariables</key>
    <dict>
        <key>PATH</key>
        <string>/usr/local/bin:/usr/bin:/bin</string>
    </dict>
</dict>
</plist>"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(plist_content.as_bytes()).unwrap();

        let changes = parse_plist_file(file.path(), "PATH").unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].value_after, "/usr/local/bin:/usr/bin:/bin");
    }

    #[test]
    fn test_parse_launchctl_setenv() {
        let plist_content = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>test.setenv</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/launchctl</string>
        <string>setenv</string>
        <string>JAVA_HOME</string>
        <string>/Library/Java/Home</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>"#;

        let mut file = NamedTempFile::new().unwrap();
        file.write_all(plist_content.as_bytes()).unwrap();

        let changes = parse_plist_file(file.path(), "JAVA_HOME").unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].value_after, "/Library/Java/Home");
    }
}
