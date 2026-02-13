use std::path::PathBuf;

/// Type of configuration file
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// /etc/environment style (KEY=value, no shell syntax)
    Environment,
    /// Shell script (bash/zsh)
    Shell,
    /// Systemd unit file
    SystemdUnit,
    /// Systemd environment.d config
    SystemdEnvironmentD,
    /// macOS launchd plist
    Plist,
}

/// A configuration file that may contain environment variable definitions
#[derive(Debug, Clone)]
pub struct ConfigFile {
    pub path: PathBuf,
    pub file_type: FileType,
    pub description: &'static str,
}

impl ConfigFile {
    pub fn new(path: impl Into<PathBuf>, file_type: FileType, description: &'static str) -> Self {
        Self {
            path: path.into(),
            file_type,
            description,
        }
    }

    pub fn shell(path: impl Into<PathBuf>, description: &'static str) -> Self {
        Self::new(path, FileType::Shell, description)
    }

    pub fn environment(path: impl Into<PathBuf>, description: &'static str) -> Self {
        Self::new(path, FileType::Environment, description)
    }

    #[cfg(target_os = "macos")]
    pub fn plist(path: impl Into<PathBuf>, description: &'static str) -> Self {
        Self::new(path, FileType::Plist, description)
    }

    #[cfg(target_os = "linux")]
    pub fn systemd_unit(path: impl Into<PathBuf>, description: &'static str) -> Self {
        Self::new(path, FileType::SystemdUnit, description)
    }

    #[cfg(target_os = "linux")]
    pub fn systemd_env_d(path: impl Into<PathBuf>, description: &'static str) -> Self {
        Self::new(path, FileType::SystemdEnvironmentD, description)
    }
}
