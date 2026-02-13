mod check;
mod compare;
mod function_trace;
mod trace;

pub use check::run_checks;
pub use compare::{compare_function, compare_variable};
pub use function_trace::{format_function_trace, format_function_trace_json};
pub use trace::{format_trace, format_trace_json};

/// Truncate a string with ellipsis if too long (Unicode-safe)
pub(crate) fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let end = s.floor_char_boundary(max_len.saturating_sub(3));
        format!("{}...", &s[..end])
    }
}
