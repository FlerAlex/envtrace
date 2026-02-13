use std::path::PathBuf;

use super::files::ConfigFile;
use crate::trace::Context;

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Get config files for a specific context on Linux
pub fn config_files_for_context(context: Context) -> Vec<ConfigFile> {
    let home = home_dir();

    match context {
        Context::InteractiveLogin => {
            let mut files = vec![
                ConfigFile::environment("/etc/environment", "PAM environment"),
                ConfigFile::shell("/etc/profile", "system profile"),
            ];

            // /etc/profile.d/*.sh (sourced by /etc/profile)
            if let Ok(entries) = std::fs::read_dir("/etc/profile.d") {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "sh") {
                        files.push(ConfigFile::shell(path, "profile.d script"));
                    }
                }
            }

            if let Some(ref h) = home {
                // First found of: ~/.bash_profile, ~/.bash_login, ~/.profile
                let bash_profile = h.join(".bash_profile");
                let bash_login = h.join(".bash_login");
                let profile = h.join(".profile");

                if bash_profile.exists() {
                    files.push(ConfigFile::shell(bash_profile, "user bash_profile"));
                } else if bash_login.exists() {
                    files.push(ConfigFile::shell(bash_login, "user bash_login"));
                } else if profile.exists() {
                    files.push(ConfigFile::shell(profile, "user profile"));
                }

                // ~/.bashrc is typically sourced by ~/.bash_profile
                files.push(ConfigFile::shell(h.join(".bashrc"), "user bashrc"));
            }

            files
        }

        Context::InteractiveNonLogin => {
            let mut files = vec![ConfigFile::environment(
                "/etc/environment",
                "PAM environment",
            )];

            // Some distros have /etc/bash.bashrc, others have /etc/bashrc
            let etc_bashrc = if PathBuf::from("/etc/bash.bashrc").exists() {
                "/etc/bash.bashrc"
            } else {
                "/etc/bashrc"
            };
            files.push(ConfigFile::shell(etc_bashrc, "system bashrc"));

            if let Some(ref h) = home {
                files.push(ConfigFile::shell(h.join(".bashrc"), "user bashrc"));
            }

            files
        }

        Context::NonInteractiveLogin | Context::NonInteractiveNonLogin => {
            // Cron and scripts get minimal environment
            vec![ConfigFile::environment(
                "/etc/environment",
                "PAM environment",
            )]
            // Note: $BASH_ENV would be sourced if set, but that's rare
        }

        Context::SystemdService => {
            // Systemd services don't source shell files
            vec![
                ConfigFile::environment("/etc/environment", "PAM environment"),
                // Unit files would be added dynamically based on the service
            ]
        }

        Context::SystemdUser => {
            let mut files = vec![ConfigFile::environment(
                "/etc/environment",
                "PAM environment",
            )];

            if let Some(ref h) = home {
                // ~/.config/environment.d/*.conf
                let env_d = h.join(".config/environment.d");
                if env_d.exists()
                    && let Ok(entries) = std::fs::read_dir(&env_d)
                {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().is_some_and(|e| e == "conf") {
                            files.push(ConfigFile::systemd_env_d(path, "systemd environment.d"));
                        }
                    }
                }
            }

            files
        }

        // macOS contexts on Linux - return empty
        _ => vec![],
    }
}

/// Get all config files that might define environment variables on Linux
pub fn all_config_files() -> Vec<ConfigFile> {
    let home = home_dir();
    let mut files = vec![
        ConfigFile::environment("/etc/environment", "PAM environment"),
        ConfigFile::shell("/etc/profile", "system profile"),
    ];

    // /etc/bash.bashrc or /etc/bashrc
    if PathBuf::from("/etc/bash.bashrc").exists() {
        files.push(ConfigFile::shell("/etc/bash.bashrc", "system bashrc"));
    } else if PathBuf::from("/etc/bashrc").exists() {
        files.push(ConfigFile::shell("/etc/bashrc", "system bashrc"));
    }

    // /etc/profile.d/*.sh
    if let Ok(entries) = std::fs::read_dir("/etc/profile.d") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "sh") {
                files.push(ConfigFile::shell(path, "profile.d script"));
            }
        }
    }

    if let Some(ref h) = home {
        files.push(ConfigFile::shell(
            h.join(".bash_profile"),
            "user bash_profile",
        ));
        files.push(ConfigFile::shell(h.join(".bash_login"), "user bash_login"));
        files.push(ConfigFile::shell(h.join(".profile"), "user profile"));
        files.push(ConfigFile::shell(h.join(".bashrc"), "user bashrc"));
        files.push(ConfigFile::shell(
            h.join(".bash_aliases"),
            "user bash_aliases",
        ));

        // systemd user environment.d
        let env_d = h.join(".config/environment.d");
        if env_d.exists()
            && let Ok(entries) = std::fs::read_dir(&env_d)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "conf") {
                    files.push(ConfigFile::systemd_env_d(path, "systemd environment.d"));
                }
            }
        }
    }

    files
}
