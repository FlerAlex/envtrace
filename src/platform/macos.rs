use std::path::PathBuf;

use super::files::ConfigFile;
use crate::trace::Context;

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Get config files for a specific context on macOS
pub fn config_files_for_context(context: Context) -> Vec<ConfigFile> {
    let home = home_dir();

    match context {
        Context::MacInteractiveLogin => {
            let mut files = vec![
                ConfigFile::shell("/etc/zshenv", "system zshenv (all zsh)"),
                ConfigFile::shell("/etc/zprofile", "system zprofile (login)"),
            ];

            if let Some(ref h) = home {
                files.push(ConfigFile::shell(
                    h.join(".zshenv"),
                    "user zshenv (all zsh)",
                ));
                files.push(ConfigFile::shell(
                    h.join(".zprofile"),
                    "user zprofile (login)",
                ));
            }

            files.push(ConfigFile::shell(
                "/etc/zshrc",
                "system zshrc (interactive)",
            ));

            if let Some(ref h) = home {
                files.push(ConfigFile::shell(
                    h.join(".zshrc"),
                    "user zshrc (interactive)",
                ));
                files.push(ConfigFile::shell(h.join(".zlogin"), "user zlogin (login)"));
            }

            files
        }

        Context::MacInteractiveNonLogin => {
            let mut files = vec![ConfigFile::shell("/etc/zshenv", "system zshenv (all zsh)")];

            if let Some(ref h) = home {
                files.push(ConfigFile::shell(
                    h.join(".zshenv"),
                    "user zshenv (all zsh)",
                ));
            }

            files.push(ConfigFile::shell(
                "/etc/zshrc",
                "system zshrc (interactive)",
            ));

            if let Some(ref h) = home {
                files.push(ConfigFile::shell(
                    h.join(".zshrc"),
                    "user zshrc (interactive)",
                ));
            }

            files
        }

        Context::MacNonInteractive => {
            let mut files = vec![ConfigFile::shell("/etc/zshenv", "system zshenv (all zsh)")];

            if let Some(ref h) = home {
                files.push(ConfigFile::shell(
                    h.join(".zshenv"),
                    "user zshenv (all zsh)",
                ));
            }

            files
        }

        Context::LaunchdAgent | Context::LaunchdDaemon => {
            // launchd doesn't source shell files - only plist files
            let mut files = Vec::new();

            if let Some(ref h) = home {
                // User LaunchAgents
                let user_agents = h.join("Library/LaunchAgents");
                if user_agents.exists()
                    && let Ok(entries) = std::fs::read_dir(&user_agents)
                {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().is_some_and(|e| e == "plist") {
                            files.push(ConfigFile::plist(path, "user LaunchAgent"));
                        }
                    }
                }
            }

            // System LaunchAgents
            let system_agents = PathBuf::from("/Library/LaunchAgents");
            if system_agents.exists()
                && let Ok(entries) = std::fs::read_dir(&system_agents)
            {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "plist") {
                        files.push(ConfigFile::plist(path, "system LaunchAgent"));
                    }
                }
            }

            files
        }

        // Linux contexts on macOS - return empty
        _ => vec![],
    }
}

/// Get all config files that might define environment variables on macOS
pub fn all_config_files() -> Vec<ConfigFile> {
    let home = home_dir();
    let mut files = vec![
        // System zsh files
        ConfigFile::shell("/etc/zshenv", "system zshenv"),
        ConfigFile::shell("/etc/zprofile", "system zprofile"),
        ConfigFile::shell("/etc/zshrc", "system zshrc"),
        // System bash files (if user uses bash)
        ConfigFile::shell("/etc/profile", "system profile"),
        ConfigFile::shell("/etc/bashrc", "system bashrc"),
    ];

    if let Some(ref h) = home {
        // User zsh files
        files.push(ConfigFile::shell(h.join(".zshenv"), "user zshenv"));
        files.push(ConfigFile::shell(h.join(".zprofile"), "user zprofile"));
        files.push(ConfigFile::shell(h.join(".zshrc"), "user zshrc"));
        files.push(ConfigFile::shell(h.join(".zlogin"), "user zlogin"));

        // User bash files
        files.push(ConfigFile::shell(
            h.join(".bash_profile"),
            "user bash_profile",
        ));
        files.push(ConfigFile::shell(h.join(".bashrc"), "user bashrc"));
        files.push(ConfigFile::shell(h.join(".profile"), "user profile"));

        // LaunchAgents
        let user_agents = h.join("Library/LaunchAgents");
        if user_agents.exists()
            && let Ok(entries) = std::fs::read_dir(&user_agents)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "plist") {
                    files.push(ConfigFile::plist(path, "user LaunchAgent"));
                }
            }
        }
    }

    files
}
