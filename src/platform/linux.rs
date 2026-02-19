use std::path::PathBuf;

use super::files::ConfigFile;
use crate::trace::Context;

fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

/// Collect environment.d/*.conf files from a directory, sorted lexicographically
fn collect_env_d_confs(dir: &std::path::Path, description: &'static str) -> Vec<ConfigFile> {
    let mut paths: Vec<PathBuf> = Vec::new();
    if dir.exists() {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|e| e == "conf") {
                    paths.push(path);
                }
            }
        }
    }
    paths.sort();
    paths
        .into_iter()
        .map(|p| ConfigFile::systemd_env_d(p, description))
        .collect()
}

/// Parse the output of `systemctl --user list-units` to extract the UWSM desktop name.
///
/// Looks for a unit named `wayland-wm@<desktop>.desktop.service` and returns the desktop name.
pub fn parse_uwsm_unit_output(output: &str) -> Option<String> {
    for line in output.lines() {
        let unit_name = line.split_whitespace().next()?;
        if let Some(rest) = unit_name.strip_prefix("wayland-wm@") {
            if let Some(desktop) = rest.strip_suffix(".desktop.service") {
                if !desktop.is_empty() {
                    return Some(desktop.to_string());
                }
            }
        }
    }
    None
}

/// Detect active UWSM session by querying systemd user units.
/// Returns the desktop name (e.g. "sway", "hyprland") if found.
fn detect_uwsm() -> Option<String> {
    let output = std::process::Command::new("systemctl")
        .args([
            "--user",
            "list-units",
            "--no-legend",
            "wayland-wm@*.desktop.service",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_uwsm_unit_output(&stdout)
}

/// Build the list of UWSM env files for a given desktop name.
///
/// Scans XDG directories in priority order (low to high):
/// `/usr/share/uwsm/` -> `/etc/xdg/uwsm/` -> `~/.config/uwsm/`
///
/// For each directory:
/// 1. `env` (base file)
/// 2. `env.d/*` (base drop-in dir, sorted)
/// 3. `env-{desktop}` (desktop-specific file)
/// 4. `env-{desktop}.d/*` (desktop-specific drop-in dir, sorted)
pub fn uwsm_env_files(desktop: &str) -> Vec<ConfigFile> {
    let home = home_dir();

    let mut xdg_dirs: Vec<PathBuf> = vec![
        PathBuf::from("/usr/share/uwsm"),
        PathBuf::from("/etc/xdg/uwsm"),
    ];
    if let Some(ref h) = home {
        xdg_dirs.push(h.join(".config/uwsm"));
    }

    let mut files = Vec::new();

    for dir in &xdg_dirs {
        // env (base file)
        let env_file = dir.join("env");
        if env_file.exists() {
            files.push(ConfigFile::shell(env_file, "uwsm env"));
        }

        // env.d/* (base drop-in, sorted)
        let env_d = dir.join("env.d");
        if env_d.exists() {
            let mut drop_in_paths: Vec<PathBuf> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&env_d) {
                for entry in entries.flatten() {
                    drop_in_paths.push(entry.path());
                }
            }
            drop_in_paths.sort();
            for path in drop_in_paths {
                files.push(ConfigFile::shell(path, "uwsm env.d"));
            }
        }

        // env-{desktop}
        let desktop_file = dir.join(format!("env-{desktop}"));
        if desktop_file.exists() {
            files.push(ConfigFile::shell(desktop_file, "uwsm env (desktop)"));
        }

        // env-{desktop}.d/* (sorted)
        let desktop_d = dir.join(format!("env-{desktop}.d"));
        if desktop_d.exists() {
            let mut drop_in_paths: Vec<PathBuf> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&desktop_d) {
                for entry in entries.flatten() {
                    drop_in_paths.push(entry.path());
                }
            }
            drop_in_paths.sort();
            for path in drop_in_paths {
                files.push(ConfigFile::shell(path, "uwsm env.d (desktop)"));
            }
        }
    }

    files
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

            // Per environment.d(5), search paths in priority order:
            files.extend(collect_env_d_confs(
                &PathBuf::from("/usr/lib/environment.d"),
                "systemd environment.d (vendor)",
            ));
            files.extend(collect_env_d_confs(
                &PathBuf::from("/run/environment.d"),
                "systemd environment.d (runtime)",
            ));
            files.extend(collect_env_d_confs(
                &PathBuf::from("/etc/environment.d"),
                "systemd environment.d (system)",
            ));

            if let Some(ref h) = home {
                files.extend(collect_env_d_confs(
                    &h.join(".config/environment.d"),
                    "systemd environment.d (user)",
                ));
            }

            files
        }

        Context::Uwsm => {
            // UWSM sessions inherit systemd user environment
            let mut files = config_files_for_context(Context::SystemdUser);

            // Then layer on uwsm-specific env files
            match detect_uwsm() {
                Some(desktop) => {
                    files.extend(uwsm_env_files(&desktop));
                }
                None => {
                    eprintln!(
                        "Warning: Could not detect active UWSM session (no wayland-wm@*.desktop.service unit found)"
                    );
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

    // System-level environment.d directories
    files.extend(collect_env_d_confs(
        &PathBuf::from("/usr/lib/environment.d"),
        "systemd environment.d (vendor)",
    ));
    files.extend(collect_env_d_confs(
        &PathBuf::from("/run/environment.d"),
        "systemd environment.d (runtime)",
    ));
    files.extend(collect_env_d_confs(
        &PathBuf::from("/etc/environment.d"),
        "systemd environment.d (system)",
    ));

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

        // User environment.d
        files.extend(collect_env_d_confs(
            &h.join(".config/environment.d"),
            "systemd environment.d (user)",
        ));

        // UWSM env files — scan all XDG dirs for any desktop
        for uwsm_dir in [
            PathBuf::from("/usr/share/uwsm"),
            PathBuf::from("/etc/xdg/uwsm"),
            h.join(".config/uwsm"),
        ] {
            if uwsm_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&uwsm_dir) {
                    let mut paths: Vec<PathBuf> = entries
                        .flatten()
                        .map(|e| e.path())
                        .filter(|p| {
                            p.file_name()
                                .and_then(|n| n.to_str())
                                .is_some_and(|n| n.starts_with("env"))
                        })
                        .collect();
                    paths.sort();
                    for path in paths {
                        if path.is_dir() {
                            // Scan drop-in directories
                            if let Ok(sub_entries) = std::fs::read_dir(&path) {
                                let mut sub_paths: Vec<PathBuf> =
                                    sub_entries.flatten().map(|e| e.path()).collect();
                                sub_paths.sort();
                                for sub_path in sub_paths {
                                    files.push(ConfigFile::shell(sub_path, "uwsm env"));
                                }
                            }
                        } else {
                            files.push(ConfigFile::shell(path, "uwsm env"));
                        }
                    }
                }
            }
        }
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parse_uwsm_unit_output_with_sway() {
        let output =
            "wayland-wm@sway.desktop.service loaded active running UWSM session: sway.desktop\n";
        assert_eq!(parse_uwsm_unit_output(output), Some("sway".to_string()));
    }

    #[test]
    fn test_parse_uwsm_unit_output_with_hyprland() {
        let output = "wayland-wm@hyprland.desktop.service loaded active running UWSM session: hyprland.desktop\n";
        assert_eq!(parse_uwsm_unit_output(output), Some("hyprland".to_string()));
    }

    #[test]
    fn test_parse_uwsm_unit_output_empty() {
        assert_eq!(parse_uwsm_unit_output(""), None);
    }

    #[test]
    fn test_parse_uwsm_unit_output_no_match() {
        let output = "some-other-service.service loaded active running Something else\n";
        assert_eq!(parse_uwsm_unit_output(output), None);
    }

    #[test]
    fn test_uwsm_env_files_no_panic() {
        // Verify it doesn't panic when no real XDG dirs exist
        let files = uwsm_env_files("nonexistent-desktop");
        assert!(
            files
                .iter()
                .all(|f| f.file_type == super::super::files::FileType::Shell)
        );
    }

    #[test]
    fn test_collect_env_d_confs_sorted() {
        let dir = TempDir::new().unwrap();
        let env_d = dir.path().join("environment.d");
        fs::create_dir_all(&env_d).unwrap();

        // Create files in non-alphabetical order
        fs::File::create(env_d.join("99-last.conf")).unwrap();
        fs::File::create(env_d.join("10-first.conf")).unwrap();
        fs::File::create(env_d.join("50-middle.conf")).unwrap();
        // Non-.conf file should be ignored
        fs::File::create(env_d.join("not-a-conf.txt")).unwrap();

        let files = collect_env_d_confs(&env_d, "test");

        assert_eq!(files.len(), 3);
        assert!(files[0].path.ends_with("10-first.conf"));
        assert!(files[1].path.ends_with("50-middle.conf"));
        assert!(files[2].path.ends_with("99-last.conf"));
    }

    #[test]
    fn test_collect_env_d_confs_nonexistent_dir() {
        let files = collect_env_d_confs(&PathBuf::from("/nonexistent/path"), "test");
        assert!(files.is_empty());
    }
}
