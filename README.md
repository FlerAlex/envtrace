# envtrace

[![CI](https://github.com/FlerAlex/envtrace/actions/workflows/ci.yml/badge.svg)](https://github.com/FlerAlex/envtrace/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-edition_2024-orange.svg)](https://www.rust-lang.org)

Trace where environment variables are defined and modified through shell startup sequences.

**envtrace** answers the question: *"Where did this environment variable come from?"*

Shell startup is complex -- variables can be set, overridden, or appended across dozens of files depending on your platform, shell, and whether you're in a login shell, a cron job, or a GUI app. envtrace walks the actual file chain and shows you exactly what happens.

## Installation

### From source

```bash
cargo install --path .
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/FlerAlex/envtrace/releases).

## Usage

### Trace a variable (default mode)

```bash
# Trace PATH through the default shell context
envtrace PATH

# Trace in a specific context
envtrace --context login PATH
envtrace --context cron PATH

# Verbose output (shows skipped files)
envtrace --verbose PATH
```

### Find all definitions

```bash
# Search all config files regardless of context
envtrace --find PATH
envtrace --find JAVA_HOME
```

### Compare across contexts

```bash
# See how a variable differs between login and cron
envtrace -C login,cron PATH
```

### Trace shell functions

```bash
# Trace a function definition
envtrace -F my_function

# Find all definitions of a function
envtrace -F --find my_function

# Compare a function across contexts
envtrace -F -C login,interactive my_function
```

### JSON output

```bash
envtrace --format json PATH
envtrace -F --format json my_function
```

### System sanity checks

```bash
# Check config files for common issues
envtrace --check
```

## Platform Support

| Platform | Shell | Contexts |
|----------|-------|----------|
| **macOS** | zsh | login, interactive, non-interactive, launchd agent/daemon |
| **Linux** | bash | login, interactive, non-interactive (cron), systemd service/user |

envtrace understands platform-specific differences:
- macOS uses `/etc/zshenv`, `/etc/zprofile`, `~/.zshrc`, etc.
- Linux uses `/etc/profile`, `/etc/profile.d/*.sh`, `~/.bashrc`, etc.
- macOS launchd agents use plist files (does not inherit shell env)
- Linux systemd services use unit files and environment.d

## Building from Source

Requires Rust edition 2024 (rustc 1.85+).

```bash
git clone https://github.com/FlerAlex/envtrace.git
cd envtrace
cargo build --release
```

The binary will be at `target/release/envtrace`.

### Running tests

```bash
cargo test
cargo fmt --check
cargo clippy
```

## License

MIT
