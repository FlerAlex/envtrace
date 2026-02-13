//! Output formatting for variable traces

use owo_colors::OwoColorize;

use crate::trace::{Operation, VariableTrace};

/// Format a variable trace as human-readable text
pub fn format_trace(trace: &VariableTrace) -> String {
    let mut output = String::new();
    let home_prefix = dirs::home_dir()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_default();

    // Header with current value
    output.push_str(&format!("{}", trace.name.bold()));

    if let Some(ref value) = trace.final_value {
        output.push_str(&format!("={}\n", value.green()));
    } else {
        output.push_str(&format!(" {}\n", "(not set)".dimmed()));
    }

    output.push('\n');

    // Context info
    output.push_str(&format!("TRACE ({}):\n", trace.context.to_string().cyan()));
    output.push_str(&format!("{}\n\n", "━".repeat(60).dimmed()));

    if trace.changes.is_empty() {
        output.push_str(&format!(
            "{}\n",
            "No modifications found in config files.".dimmed()
        ));

        if trace.final_value.is_some() {
            output.push_str(&format!(
                "{}\n",
                "Value may be inherited from parent process or set by the system.".dimmed()
            ));
        }
    } else {
        // List each change
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
            let effect = describe_effect(
                change.operation,
                &change.value_after,
                change.value_before.as_deref(),
            );
            output.push_str(&format!("    {} {}\n", "→".green(), effect));

            output.push('\n');
        }
    }

    // Final value
    if let Some(ref value) = trace.final_value {
        output.push_str(&format!("{} {}\n", "FINAL:".bold(), value.green()));
    }

    output
}

/// Describe the effect of an operation
fn describe_effect(operation: Operation, value_after: &str, value_before: Option<&str>) -> String {
    match operation {
        Operation::Set | Operation::Export => {
            if value_before.is_some() {
                format!("sets to \"{}\"", super::truncate(value_after, 60))
            } else {
                format!("initializes to \"{}\"", super::truncate(value_after, 60))
            }
        }
        Operation::Append => {
            if let Some(before) = value_before
                && value_after.starts_with(before)
                && value_after.len() > before.len()
            {
                let appended = &value_after[before.len()..].trim_start_matches(':');
                return format!("appends \"{}\"", appended);
            }
            "appends to value".to_string()
        }
        Operation::Prepend => {
            if let Some(before) = value_before
                && value_after.ends_with(before)
                && value_after.len() > before.len()
            {
                let prepended =
                    &value_after[..value_after.len() - before.len()].trim_end_matches(':');
                return format!("prepends \"{}\"", prepended);
            }
            "prepends to value".to_string()
        }
        Operation::Unset => "unsets the variable".to_string(),
        Operation::Conditional => {
            format!(
                "conditionally sets to \"{}\"",
                super::truncate(value_after, 60)
            )
        }
    }
}

/// Format a variable trace as JSON
pub fn format_trace_json(trace: &VariableTrace) -> String {
    serde_json::to_string_pretty(trace).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::{Context, VariableChange};
    use std::path::PathBuf;

    #[test]
    fn test_format_trace_empty() {
        let trace = VariableTrace {
            name: "TEST".to_string(),
            final_value: None,
            changes: vec![],
            context: Context::MacInteractiveLogin,
        };

        let output = format_trace(&trace);
        assert!(output.contains("TEST"));
        assert!(output.contains("not set"));
        assert!(output.contains("No modifications found"));
    }

    #[test]
    fn test_format_trace_with_changes() {
        let trace = VariableTrace {
            name: "PATH".to_string(),
            final_value: Some("/usr/local/bin:/usr/bin".to_string()),
            changes: vec![
                VariableChange {
                    file: PathBuf::from("/etc/zprofile"),
                    line_number: 5,
                    line_content: "export PATH=/usr/bin".to_string(),
                    operation: Operation::Export,
                    value_before: None,
                    value_after: "/usr/bin".to_string(),
                },
                VariableChange {
                    file: PathBuf::from("/Users/test/.zshrc"),
                    line_number: 10,
                    line_content: r#"export PATH="/usr/local/bin:$PATH""#.to_string(),
                    operation: Operation::Prepend,
                    value_before: Some("/usr/bin".to_string()),
                    value_after: "/usr/local/bin:/usr/bin".to_string(),
                },
            ],
            context: Context::MacInteractiveLogin,
        };

        let output = format_trace(&trace);
        assert!(output.contains("PATH"));
        assert!(output.contains("/etc/zprofile"));
        assert!(output.contains("[1]"));
        assert!(output.contains("[2]"));
        assert!(output.contains("FINAL:"));
    }

    #[test]
    fn test_format_trace_json() {
        let trace = VariableTrace {
            name: "TEST".to_string(),
            final_value: Some("value".to_string()),
            changes: vec![],
            context: Context::MacInteractiveLogin,
        };

        let json = format_trace_json(&trace);
        assert!(json.contains("\"name\": \"TEST\""));
        assert!(json.contains("\"final_value\": \"value\""));
    }
}
