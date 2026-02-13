//! Output formatting for function traces

use owo_colors::OwoColorize;

use crate::trace::function::{FunctionOperation, FunctionTrace};

/// Format a function trace as human-readable text
pub fn format_function_trace(trace: &FunctionTrace) -> String {
    let mut output = String::new();
    let home_prefix = dirs::home_dir()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default();

    // Header
    output.push_str(&format!("{}() ", trace.name.bold()));
    output.push_str(&format!("{}\n", "[function]".dimmed()));
    output.push('\n');

    // Context info
    output.push_str(&format!("TRACE ({}):\n", trace.context.to_string().cyan()));
    output.push_str(&format!("{}\n\n", "━".repeat(60).dimmed()));

    if trace.changes.is_empty() {
        output.push_str(&format!(
            "{}\n",
            "No function definitions found in config files.".dimmed()
        ));

        if trace.is_defined {
            output.push_str(&format!(
                "{}\n",
                "Function exists in current shell but definition source not found.".dimmed()
            ));
        }
    } else {
        for (i, change) in trace.changes.iter().enumerate() {
            let num = format!("[{}]", i + 1);

            // File and line number
            let file_display = change.file.to_string_lossy().replace(&home_prefix, "~");

            output.push_str(&format!(
                "{} {}:{}\n",
                num.yellow(),
                file_display.blue(),
                change.line_number
            ));

            // The actual line content
            output.push_str(&format!("    {}\n", change.line_content.dimmed()));

            // Effect description
            match change.operation {
                FunctionOperation::Define => {
                    let line_info = if change.body_lines > 0 {
                        format!(" ({} lines)", change.body_lines)
                    } else {
                        String::new()
                    };
                    output.push_str(&format!(
                        "    {} {}{}\n",
                        "→".green(),
                        "defines function",
                        line_info
                    ));

                    // Show body preview
                    if let Some(ref body) = change.body {
                        for line in body.lines().take(MAX_BODY_PREVIEW) {
                            output.push_str(&format!("       {}\n", line.dimmed()));
                        }
                        if body.lines().count() > MAX_BODY_PREVIEW {
                            output.push_str(&format!("       {}\n", "...".dimmed()));
                        }
                    }
                }
                FunctionOperation::Autoload => {
                    output.push_str(&format!(
                        "    {} {}\n",
                        "→".green(),
                        "autoloads function (lazy-loaded on first call)"
                    ));
                }
                FunctionOperation::Unset => {
                    output.push_str(&format!("    {} {}\n", "→".green(), "removes function"));
                }
            }

            output.push('\n');
        }
    }

    // Final status
    output.push_str(&format!(
        "{} {}\n",
        "DEFINED:".bold(),
        if trace.is_defined {
            "yes".green().to_string()
        } else {
            "no".red().to_string()
        }
    ));

    output
}

const MAX_BODY_PREVIEW: usize = 5;

/// Format a function trace as JSON
pub fn format_function_trace_json(trace: &FunctionTrace) -> String {
    serde_json::to_string_pretty(trace).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::Context;
    use crate::trace::function::{FunctionChange, FunctionOperation};
    use std::path::PathBuf;

    #[test]
    fn test_format_empty_trace() {
        let trace = FunctionTrace {
            name: "my_func".to_string(),
            is_defined: false,
            changes: vec![],
            context: Context::MacInteractiveLogin,
        };

        let output = format_function_trace(&trace);
        assert!(output.contains("my_func"));
        assert!(output.contains("No function definitions found"));
        assert!(output.contains("no"));
    }

    #[test]
    fn test_format_trace_with_definition() {
        let trace = FunctionTrace {
            name: "my_func".to_string(),
            is_defined: true,
            changes: vec![FunctionChange {
                file: PathBuf::from("/Users/test/.zshrc"),
                line_number: 10,
                line_content: "my_func() {".to_string(),
                operation: FunctionOperation::Define,
                body: Some("    echo hello\n}".to_string()),
                body_lines: 3,
            }],
            context: Context::MacInteractiveLogin,
        };

        let output = format_function_trace(&trace);
        assert!(output.contains("my_func"));
        assert!(output.contains("[1]"));
        assert!(output.contains("defines function"));
        assert!(output.contains("3 lines"));
        assert!(output.contains("yes"));
    }

    #[test]
    fn test_format_trace_with_autoload() {
        let trace = FunctionTrace {
            name: "compinit".to_string(),
            is_defined: true,
            changes: vec![FunctionChange {
                file: PathBuf::from("/etc/zshrc"),
                line_number: 5,
                line_content: "autoload -Uz compinit".to_string(),
                operation: FunctionOperation::Autoload,
                body: None,
                body_lines: 0,
            }],
            context: Context::MacInteractiveLogin,
        };

        let output = format_function_trace(&trace);
        assert!(output.contains("compinit"));
        assert!(output.contains("autoloads function"));
        assert!(output.contains("yes"));
    }

    #[test]
    fn test_format_json() {
        let trace = FunctionTrace {
            name: "my_func".to_string(),
            is_defined: true,
            changes: vec![],
            context: Context::MacInteractiveLogin,
        };

        let json = format_function_trace_json(&trace);
        assert!(json.contains("\"name\": \"my_func\""));
        assert!(json.contains("\"is_defined\": true"));
    }
}
