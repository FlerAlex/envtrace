pub(crate) mod common;
mod environment;
mod plist;
mod shell;
mod shell_function;

pub use environment::parse_environment_file;
pub use plist::{launchctl_getenv, parse_plist_file};
pub use shell::{ParsedShellEntry, parse_shell_file};
pub use shell_function::{ParsedFunctionEntry, parse_shell_file_for_function};
