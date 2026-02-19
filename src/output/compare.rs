//! Comparison output formatting for showing a variable across contexts

use tabled::{Table, Tabled};

use crate::platform::Platform;
use crate::trace::function::FunctionOperation;
use crate::trace::{Context, TraceEngine};

#[derive(Tabled)]
struct ContextRow {
    #[tabled(rename = "Context")]
    context: String,
    #[tabled(rename = "Value")]
    value: String,
}

/// Compare a variable's value across multiple contexts
pub fn compare_variable(
    engine: &mut TraceEngine,
    var_name: &str,
    context_names: &[String],
    platform: Platform,
) -> String {
    let contexts: Vec<Context> = context_names
        .iter()
        .filter_map(|name| parse_context_name(name, platform))
        .collect();

    if contexts.is_empty() {
        return "No valid contexts specified.".to_string();
    }

    let mut rows = Vec::new();

    for context in contexts {
        let trace = engine.trace(var_name, context);
        let value = trace.final_value.unwrap_or_else(|| "(not set)".to_string());

        rows.push(ContextRow {
            context: context.to_string(),
            value: super::truncate(&value, 60),
        });
    }

    let table = Table::new(rows).to_string();

    format!("Comparing {} across contexts:\n\n{}", var_name, table)
}

/// Parse a context name string into a Context enum
fn parse_context_name(name: &str, platform: Platform) -> Option<Context> {
    match (name.to_lowercase().as_str(), platform) {
        ("login", Platform::MacOS) => Some(Context::MacInteractiveLogin),
        ("login", Platform::Linux) => Some(Context::InteractiveLogin),
        ("interactive", Platform::MacOS) => Some(Context::MacInteractiveNonLogin),
        ("interactive", Platform::Linux) => Some(Context::InteractiveNonLogin),
        ("cron", _) => Some(Context::NonInteractiveNonLogin),
        ("systemd", _) => Some(Context::SystemdService),
        ("systemd-user", _) => Some(Context::SystemdUser),
        ("uwsm", _) => Some(Context::Uwsm),
        ("launchd", _) => Some(Context::LaunchdAgent),
        ("noninteractive", Platform::MacOS) => Some(Context::MacNonInteractive),
        ("noninteractive", Platform::Linux) => Some(Context::NonInteractiveNonLogin),
        _ => None,
    }
}

/// Compare a function's definition across multiple contexts
pub fn compare_function(
    engine: &mut TraceEngine,
    func_name: &str,
    context_names: &[String],
    platform: Platform,
) -> String {
    let contexts: Vec<Context> = context_names
        .iter()
        .filter_map(|name| parse_context_name(name, platform))
        .collect();

    if contexts.is_empty() {
        return "No valid contexts specified.".to_string();
    }

    let mut rows = Vec::new();

    for context in contexts {
        let trace = engine.trace_function(func_name, context);
        let value = if trace.changes.is_empty() {
            "not defined".to_string()
        } else {
            // Use the last change to determine status
            match trace.changes.last().unwrap().operation {
                FunctionOperation::Define => {
                    let lines = trace.changes.last().unwrap().body_lines;
                    format!("defined ({} lines)", lines)
                }
                FunctionOperation::Autoload => "autoloaded".to_string(),
                FunctionOperation::Unset => "not defined".to_string(),
            }
        };

        rows.push(ContextRow {
            context: context.to_string(),
            value,
        });
    }

    let table = Table::new(rows).to_string();

    format!("Comparing {}() across contexts:\n\n{}", func_name, table)
}
