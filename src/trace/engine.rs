//! Main tracing engine that coordinates file discovery and parsing

use std::collections::HashSet;
use std::env;
use std::path::PathBuf;

use crate::files::discover_files;
use crate::parser::{
    ParsedFunctionEntry, ParsedShellEntry, parse_environment_file, parse_plist_file,
    parse_shell_file, parse_shell_file_for_function,
};
use crate::platform::{ConfigFile, FileType, Platform};
use crate::trace::function::{FunctionChange, FunctionOperation, FunctionTrace};
use crate::trace::{Context, Operation, VariableChange, VariableTrace};

/// Configuration for the tracing engine
pub struct TraceConfig {
    /// Follow source/. commands in shell scripts
    pub follow_sources: bool,
    /// Include verbose information about skipped files
    pub verbose: bool,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            follow_sources: true,
            verbose: false,
        }
    }
}

/// The main tracing engine
pub struct TraceEngine {
    platform: Platform,
    config: TraceConfig,
    /// Track sourced files to prevent infinite loops
    sourced_files: HashSet<PathBuf>,
}

impl TraceEngine {
    pub fn new(platform: Platform) -> Self {
        Self {
            platform,
            config: TraceConfig::default(),
            sourced_files: HashSet::new(),
        }
    }

    pub fn with_config(mut self, config: TraceConfig) -> Self {
        self.config = config;
        self
    }

    /// Trace a variable through the startup sequence for a given context
    pub fn trace(&mut self, var_name: &str, context: Context) -> VariableTrace {
        self.sourced_files.clear();

        let files = discover_files(self.platform, context);
        let mut changes: Vec<VariableChange> = Vec::new();
        let mut current_value: Option<String> = None;

        // Get the current environment value as a starting point reference
        let env_value = env::var(var_name).ok();

        for config_file in files {
            self.process_file(&config_file, var_name, &mut current_value, &mut changes);
        }

        VariableTrace {
            name: var_name.to_string(),
            final_value: current_value.or(env_value),
            changes,
            context,
        }
    }

    /// Find all definitions of a variable across all config files
    pub fn find_all(&mut self, var_name: &str) -> Vec<VariableChange> {
        self.sourced_files.clear();

        let files = self.platform.all_config_files();
        let mut changes: Vec<VariableChange> = Vec::new();
        let mut current_value: Option<String> = None;

        for config_file in files {
            if config_file.path.exists() {
                self.process_file(&config_file, var_name, &mut current_value, &mut changes);
            }
        }

        changes
    }

    /// Trace a function through the startup sequence for a given context
    pub fn trace_function(&mut self, func_name: &str, context: Context) -> FunctionTrace {
        self.sourced_files.clear();

        let files = discover_files(self.platform, context);
        let mut changes: Vec<FunctionChange> = Vec::new();

        for config_file in files {
            self.process_file_for_function(&config_file, func_name, &mut changes);
        }

        // Determine if function is defined based on the last change
        let is_defined = if changes.is_empty() {
            self.check_function_exists(func_name)
        } else {
            changes
                .last()
                .map(|c| c.operation != FunctionOperation::Unset)
                .unwrap_or(false)
        };

        FunctionTrace {
            name: func_name.to_string(),
            is_defined,
            changes,
            context,
        }
    }

    /// Find all definitions of a function across all config files
    pub fn find_all_functions(&mut self, func_name: &str) -> Vec<FunctionChange> {
        self.sourced_files.clear();

        let files = self.platform.all_config_files();
        let mut changes: Vec<FunctionChange> = Vec::new();

        for config_file in files {
            if config_file.path.exists() {
                self.process_file_for_function(&config_file, func_name, &mut changes);
            }
        }

        changes
    }

    fn process_file_for_function(
        &mut self,
        config_file: &ConfigFile,
        func_name: &str,
        changes: &mut Vec<FunctionChange>,
    ) {
        // Only shell files can contain function definitions
        if config_file.file_type != FileType::Shell {
            return;
        }

        // Prevent infinite loops from circular sources
        let canonical = config_file
            .path
            .canonicalize()
            .unwrap_or(config_file.path.clone());
        if self.sourced_files.contains(&canonical) {
            return;
        }
        self.sourced_files.insert(canonical);

        match parse_shell_file_for_function(&config_file.path, func_name) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        ParsedFunctionEntry::Source(source_path) => {
                            if self.config.follow_sources {
                                let source_file = ConfigFile::shell(source_path, "sourced file");
                                self.process_file_for_function(&source_file, func_name, changes);
                            }
                        }
                        ParsedFunctionEntry::Definition(change) => {
                            changes.push(change);
                        }
                    }
                }
            }
            Err(e) => {
                if self.config.verbose {
                    eprintln!(
                        "Warning: Could not read {}: {}",
                        config_file.path.display(),
                        e
                    );
                }
            }
        }
    }

    /// Check if a function exists in the current shell (best-effort)
    pub fn check_function_exists(&self, func_name: &str) -> bool {
        // Validate function name to prevent command injection
        if func_name.is_empty()
            || !func_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            return false;
        }

        let (shell, cmd) = if cfg!(target_os = "macos") {
            ("zsh", format!("whence -w {}", func_name))
        } else {
            ("bash", format!("type -t {}", func_name))
        };

        std::process::Command::new(shell)
            .args(["-c", &cmd])
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    fn process_file(
        &mut self,
        config_file: &ConfigFile,
        var_name: &str,
        current_value: &mut Option<String>,
        changes: &mut Vec<VariableChange>,
    ) {
        // Prevent infinite loops from circular sources
        let canonical = config_file
            .path
            .canonicalize()
            .unwrap_or(config_file.path.clone());
        if self.sourced_files.contains(&canonical) {
            return;
        }
        self.sourced_files.insert(canonical);

        let result = match config_file.file_type {
            FileType::Environment => parse_environment_file(&config_file.path, var_name),
            FileType::Shell => parse_shell_file(
                &config_file.path,
                var_name,
                current_value.as_deref(),
            )
            .map(|entries| {
                let mut file_changes = Vec::new();
                for entry in entries {
                    match entry {
                        ParsedShellEntry::Source(source_path) => {
                            if self.config.follow_sources {
                                let source_file = ConfigFile::shell(source_path, "sourced file");
                                self.process_file(&source_file, var_name, current_value, changes);
                            }
                        }
                        ParsedShellEntry::Assignment(change) => {
                            file_changes.push(change);
                        }
                    }
                }
                file_changes
            }),
            FileType::Plist => parse_plist_file(&config_file.path, var_name),
            FileType::SystemdUnit | FileType::SystemdEnvironmentD => {
                // TODO: Implement systemd parsers
                Ok(vec![])
            }
        };

        match result {
            Ok(file_changes) => {
                for mut change in file_changes {
                    // Update value_before with the current tracked value
                    change.value_before = current_value.clone();

                    // Update the current value based on the operation
                    match change.operation {
                        Operation::Unset => {
                            *current_value = None;
                        }
                        _ => {
                            *current_value = Some(change.value_after.clone());
                        }
                    }

                    changes.push(change);
                }
            }
            Err(e) => {
                if self.config.verbose {
                    eprintln!(
                        "Warning: Could not read {}: {}",
                        config_file.path.display(),
                        e
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn test_trace_function_simple() {
        let dir = TempDir::new().unwrap();
        let zshrc = create_test_file(&dir, ".zshrc", "my_func() {\n    echo hello\n}\n");

        let config = ConfigFile::shell(zshrc, "test zshrc");
        let mut engine = TraceEngine::new(Platform::detect());
        let mut changes = Vec::new();

        engine.process_file_for_function(&config, "my_func", &mut changes);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].operation, FunctionOperation::Define);
        assert!(changes[0].body.is_some());
    }

    #[test]
    fn test_trace_function_override() {
        let dir = TempDir::new().unwrap();
        let file1 = create_test_file(&dir, "file1.sh", "my_func() {\n    echo first\n}\n");
        let file2 = create_test_file(&dir, "file2.sh", "my_func() {\n    echo second\n}\n");

        let mut engine = TraceEngine::new(Platform::detect());
        let mut changes = Vec::new();

        let config1 = ConfigFile::shell(file1, "file1");
        let config2 = ConfigFile::shell(file2, "file2");
        engine.process_file_for_function(&config1, "my_func", &mut changes);
        engine.process_file_for_function(&config2, "my_func", &mut changes);

        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].operation, FunctionOperation::Define);
        assert_eq!(changes[1].operation, FunctionOperation::Define);
    }

    #[test]
    fn test_trace_function_define_then_unset() {
        let dir = TempDir::new().unwrap();
        let file = create_test_file(
            &dir,
            "test.sh",
            "my_func() {\n    echo hello\n}\nunset -f my_func\n",
        );

        let mut engine = TraceEngine::new(Platform::detect());
        let mut changes = Vec::new();

        let config = ConfigFile::shell(file, "test");
        engine.process_file_for_function(&config, "my_func", &mut changes);

        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].operation, FunctionOperation::Define);
        assert_eq!(changes[1].operation, FunctionOperation::Unset);
    }

    #[test]
    fn test_trace_simple_export() {
        let dir = TempDir::new().unwrap();
        let bashrc = create_test_file(&dir, ".bashrc", "export TEST_VAR=hello\n");

        // Create a mock config file
        let config = ConfigFile::shell(bashrc, "test bashrc");

        let mut engine = TraceEngine::new(Platform::detect());
        let mut current_value = None;
        let mut changes = Vec::new();

        engine.process_file(&config, "TEST_VAR", &mut current_value, &mut changes);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].value_after, "hello");
        assert_eq!(current_value, Some("hello".to_string()));
    }

    #[test]
    fn test_trace_append() {
        let dir = TempDir::new().unwrap();
        let bashrc = create_test_file(&dir, ".bashrc", r#"export PATH="$PATH:/new/path""#);

        let config = ConfigFile::shell(bashrc, "test bashrc");

        let mut engine = TraceEngine::new(Platform::detect());
        let mut current_value = Some("/usr/bin".to_string());
        let mut changes = Vec::new();

        engine.process_file(&config, "PATH", &mut current_value, &mut changes);

        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0].operation, Operation::Append);
        assert_eq!(changes[0].value_after, "/usr/bin:/new/path");
    }
}
