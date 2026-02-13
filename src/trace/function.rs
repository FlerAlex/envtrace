use std::path::PathBuf;

use serde::Serialize;

use super::Context;

/// The type of operation performed on a shell function
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FunctionOperation {
    /// Function defined with body (POSIX, bash, or hybrid syntax)
    Define,
    /// Zsh autoload declaration
    Autoload,
    /// unset -f func_name
    Unset,
}

/// Represents a single modification to a shell function
#[derive(Debug, Clone, Serialize)]
pub struct FunctionChange {
    pub file: PathBuf,
    pub line_number: usize,
    pub line_content: String,
    pub operation: FunctionOperation,
    /// The function body (first few lines), None for autoload/unset
    pub body: Option<String>,
    /// Total number of lines in the function body
    pub body_lines: usize,
}

/// Represents the full trace of a function through the startup sequence
#[derive(Debug, Serialize)]
pub struct FunctionTrace {
    pub name: String,
    pub is_defined: bool,
    pub changes: Vec<FunctionChange>,
    pub context: Context,
}

impl std::fmt::Display for FunctionOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FunctionOperation::Define => write!(f, "define"),
            FunctionOperation::Autoload => write!(f, "autoload"),
            FunctionOperation::Unset => write!(f, "unset"),
        }
    }
}
