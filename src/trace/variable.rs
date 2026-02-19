use std::path::PathBuf;

use serde::Serialize;

/// Represents a single modification to a variable
#[derive(Debug, Clone, Serialize)]
pub struct VariableChange {
    pub file: PathBuf,
    pub line_number: usize,
    pub line_content: String,
    pub operation: Operation,
    pub value_before: Option<String>,
    pub value_after: String,
}

/// The type of operation performed on a variable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    /// VAR=value (not exported)
    Set,
    /// export VAR=value
    Export,
    /// VAR="$VAR:new" or PATH="$PATH:/new/path"
    Append,
    /// VAR="new:$VAR" or PATH="/new/path:$PATH"
    Prepend,
    /// unset VAR
    Unset,
    /// [ -f x ] && export VAR=y (conditional assignment)
    Conditional,
}

/// Represents the full trace of a variable through the startup sequence
#[derive(Debug, Serialize)]
pub struct VariableTrace {
    pub name: String,
    pub final_value: Option<String>,
    pub changes: Vec<VariableChange>,
    pub context: Context,
}

/// Shell context determines which files are sourced
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Context {
    // Linux contexts
    /// Interactive login shell (SSH, console login)
    InteractiveLogin,
    /// Interactive non-login shell (new terminal window)
    InteractiveNonLogin,
    /// Non-interactive login shell (rare)
    NonInteractiveLogin,
    /// Non-interactive non-login shell (cron, scripts)
    NonInteractiveNonLogin,
    /// Systemd system service
    SystemdService,
    /// Systemd user service
    SystemdUser,
    /// UWSM Wayland compositor session
    Uwsm,

    // macOS contexts
    /// macOS interactive login shell (zsh default)
    MacInteractiveLogin,
    /// macOS interactive non-login shell (new terminal tab)
    MacInteractiveNonLogin,
    /// macOS non-interactive shell (scripts)
    MacNonInteractive,
    /// macOS launchd agent (GUI apps, user services)
    LaunchdAgent,
    /// macOS launchd daemon (system services)
    LaunchdDaemon,
}

impl Context {
    /// Returns true if this is a macOS-specific context
    pub fn is_macos(&self) -> bool {
        matches!(
            self,
            Context::MacInteractiveLogin
                | Context::MacInteractiveNonLogin
                | Context::MacNonInteractive
                | Context::LaunchdAgent
                | Context::LaunchdDaemon
        )
    }

    /// Returns true if this is a Linux-specific context
    pub fn is_linux(&self) -> bool {
        !self.is_macos()
    }

    /// Returns a human-readable description of the context
    pub fn description(&self) -> &'static str {
        match self {
            Context::InteractiveLogin => "interactive login shell",
            Context::InteractiveNonLogin => "interactive non-login shell",
            Context::NonInteractiveLogin => "non-interactive login shell",
            Context::NonInteractiveNonLogin => "non-interactive shell (cron, scripts)",
            Context::SystemdService => "systemd system service",
            Context::SystemdUser => "systemd user service",
            Context::Uwsm => "UWSM Wayland session",
            Context::MacInteractiveLogin => "zsh interactive login shell",
            Context::MacInteractiveNonLogin => "zsh interactive non-login shell",
            Context::MacNonInteractive => "zsh non-interactive shell",
            Context::LaunchdAgent => "launchd agent (GUI apps)",
            Context::LaunchdDaemon => "launchd daemon (system service)",
        }
    }

    /// Returns the default context for the current platform
    pub fn default_for_platform() -> Self {
        if cfg!(target_os = "macos") {
            Context::MacInteractiveLogin
        } else {
            Context::InteractiveLogin
        }
    }
}

impl std::fmt::Display for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Set => write!(f, "set"),
            Operation::Export => write!(f, "export"),
            Operation::Append => write!(f, "append"),
            Operation::Prepend => write!(f, "prepend"),
            Operation::Unset => write!(f, "unset"),
            Operation::Conditional => write!(f, "conditional"),
        }
    }
}
