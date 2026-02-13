mod files;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

pub use files::{ConfigFile, FileType};

/// Detected platform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Linux,
    MacOS,
}

impl Platform {
    /// Detect the current platform at runtime
    pub fn detect() -> Self {
        if cfg!(target_os = "macos") {
            Platform::MacOS
        } else {
            Platform::Linux
        }
    }

    /// Get the platform-specific config files for a given context
    pub fn config_files(&self, context: crate::trace::Context) -> Vec<ConfigFile> {
        match self {
            #[cfg(target_os = "linux")]
            Platform::Linux => linux::config_files_for_context(context),
            #[cfg(target_os = "macos")]
            Platform::MacOS => macos::config_files_for_context(context),
            // Catch-all for cross-compilation scenarios
            _ => vec![],
        }
    }

    /// Get all config files that might define environment variables
    pub fn all_config_files(&self) -> Vec<ConfigFile> {
        match self {
            #[cfg(target_os = "linux")]
            Platform::Linux => linux::all_config_files(),
            #[cfg(target_os = "macos")]
            Platform::MacOS => macos::all_config_files(),
            // Catch-all for cross-compilation scenarios
            _ => vec![],
        }
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Platform::Linux => write!(f, "linux"),
            Platform::MacOS => write!(f, "macos"),
        }
    }
}
