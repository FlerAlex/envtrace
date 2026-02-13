//! Environment sanity checks

use std::env;
use std::path::PathBuf;

use owo_colors::OwoColorize;

use crate::platform::Platform;

/// Run environment sanity checks
pub fn run_checks(platform: Platform, verbose: bool) -> String {
    let mut output = String::new();
    let mut issues_found = 0;

    output.push_str(&format!("{}\n\n", "Environment Health Check".bold()));

    // Check PATH
    output.push_str(&format!("{}\n", "PATH Analysis:".cyan()));
    output.push_str(&format!("{}\n", "─".repeat(40).dimmed()));

    if let Ok(path) = env::var("PATH") {
        let entries: Vec<&str> = path.split(':').collect();

        // Check for non-existent directories
        let mut missing = Vec::new();
        for entry in &entries {
            if !entry.is_empty() {
                let path = PathBuf::from(entry);
                if !path.exists() {
                    missing.push(entry.to_string());
                }
            }
        }

        if !missing.is_empty() {
            issues_found += missing.len();
            output.push_str(&format!(
                "\n{} Non-existent directories in PATH:\n",
                "!".yellow()
            ));
            for dir in &missing {
                output.push_str(&format!("  {} {}\n", "-".red(), dir));
            }
        }

        // Check for duplicates
        let mut seen = std::collections::HashSet::new();
        let mut duplicates = Vec::new();
        for entry in &entries {
            if !entry.is_empty() && !seen.insert(*entry) {
                duplicates.push(entry.to_string());
            }
        }

        if !duplicates.is_empty() {
            issues_found += duplicates.len();
            output.push_str(&format!("\n{} Duplicate entries in PATH:\n", "!".yellow()));
            for dir in duplicates.iter().collect::<std::collections::HashSet<_>>() {
                output.push_str(&format!("  {} {}\n", "-".yellow(), dir));
            }
        }

        // Check for empty entries
        let empty_count = entries.iter().filter(|e| e.is_empty()).count();
        if empty_count > 0 {
            issues_found += 1;
            output.push_str(&format!(
                "\n{} PATH contains {} empty entries (::)\n",
                "!".yellow(),
                empty_count
            ));
        }

        if verbose {
            output.push_str(&format!("\n{}\n", "PATH entries:".dimmed()));
            for (i, entry) in entries.iter().enumerate() {
                let status = if entry.is_empty() {
                    "(empty)".yellow().to_string()
                } else if PathBuf::from(entry).exists() {
                    "OK".green().to_string()
                } else {
                    "MISSING".red().to_string()
                };
                output.push_str(&format!("  {:2}. {} [{}]\n", i + 1, entry, status));
            }
        }
    } else {
        issues_found += 1;
        output.push_str(&format!("{} PATH is not set!\n", "X".red()));
    }

    // macOS-specific: check for launchd differences
    #[cfg(target_os = "macos")]
    {
        output.push_str(&format!("\n\n{}\n", "launchd Environment:".cyan()));
        output.push_str(&format!("{}\n", "─".repeat(40).dimmed()));

        if let Some(launchd_path) = crate::parser::launchctl_getenv("PATH") {
            let shell_path = env::var("PATH").unwrap_or_default();
            if launchd_path != shell_path {
                issues_found += 1;
                output.push_str(&format!(
                    "\n{} PATH differs between shell and launchd (GUI apps):\n",
                    "!".yellow()
                ));
                output.push_str(&format!(
                    "  Shell:   {}\n",
                    super::truncate(&shell_path, 50)
                ));
                output.push_str(&format!(
                    "  launchd: {}\n",
                    super::truncate(&launchd_path, 50)
                ));
                output.push_str(&format!(
                    "\n  {} GUI apps won't see PATH entries added in shell config files.\n",
                    "TIP:".cyan()
                ));
                output.push_str("  To fix, create ~/Library/LaunchAgents/environment.plist\n");
            } else {
                output.push_str(&format!(
                    "{} PATH is the same in shell and launchd\n",
                    "OK".green()
                ));
            }
        } else {
            output.push_str(&format!(
                "{} Could not query launchd PATH (this is normal on some systems)\n",
                "-".dimmed()
            ));
        }
    }

    // Summary
    output.push_str(&format!("\n\n{}\n", "─".repeat(40).dimmed()));
    if issues_found == 0 {
        output.push_str(&format!("{} No issues found.\n", "OK".green()));
    } else {
        output.push_str(&format!(
            "{} {} issue(s) found.\n",
            "!".yellow(),
            issues_found
        ));
    }

    let _ = platform; // silence unused warning on non-macOS

    output
}
