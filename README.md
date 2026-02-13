# envtrace

[![CI](https://github.com/FlerAlex/envtrace/actions/workflows/ci.yml/badge.svg)](https://github.com/FlerAlex/envtrace/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![crates.io](https://img.shields.io/crates/v/envtrace.svg)](https://crates.io/crates/envtrace)
[![Rust](https://img.shields.io/badge/rust-edition_2024-orange.svg)](https://www.rust-lang.org)

Trace where environment variables are defined and modified through shell startup sequences.

**envtrace** answers the question: *"Where did this environment variable come from?"*

Shell startup is complex -- variables can be set, overridden, or appended across dozens of files depending on your platform, shell, and whether you're in a login shell, a cron job, or a GUI app. envtrace walks the actual file chain and shows you exactly what happens.

## Installation

### From crates.io

```bash
cargo install envtrace
```

### Pre-built binaries

Download from [GitHub Releases](https://github.com/FlerAlex/envtrace/releases).

## Usage

### Trace a variable

The default mode traces a variable through the shell startup sequence and shows every file that touches it:

```bash
envtrace PATH
```

```
PATH=/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin

TRACE (macOS Interactive Login):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[1] /etc/zprofile:5
    export PATH=/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin
    → initializes to "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"

[2] ~/.zshrc:12
    export PATH="/opt/homebrew/bin:$PATH"
    → prepends "/opt/homebrew/bin"

FINAL: /opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin
```

Trace in a specific shell context:

```bash
envtrace --context login PATH     # login shell
envtrace --context interactive PATH  # non-login interactive shell
envtrace --context cron PATH     # cron jobs / scripts
envtrace --context launchd PATH  # macOS GUI apps (launchd agent)
envtrace --context systemd PATH  # Linux systemd services
```

Use `--verbose` to see which files were checked but had no matches:

```bash
envtrace --verbose JAVA_HOME
```

### Find all definitions

Search across all config files regardless of context -- useful when you're not sure where a variable is set:

```bash
envtrace --find JAVA_HOME
```

```
Found 2 definition(s) of JAVA_HOME:

  ~/.zprofile:8
    export JAVA_HOME=/Library/Java/JavaVirtualMachines/zulu-17.jdk/Contents/Home

  ~/.zshrc:45
    export JAVA_HOME=$(/usr/libexec/java_home)
```

### Compare across contexts

See how a variable's final value differs between shell contexts. This is especially useful for diagnosing "it works in my terminal but not in cron" problems:

```bash
envtrace -C login,cron PATH
```

```
Comparing PATH across contexts:

+----------------------------+----------------------------------------------+
| Context                    | Value                                        |
+----------------------------+----------------------------------------------+
| macOS Interactive Login    | /opt/homebrew/bin:/usr/local/bin:/usr/bin:... |
+----------------------------+----------------------------------------------+
| Non-Interactive Non-Login  | /usr/bin:/bin:/usr/sbin:/sbin                |
+----------------------------+----------------------------------------------+
```

Available context names: `login`, `interactive`, `cron`, `launchd`, `systemd`, `noninteractive`.

### Trace shell functions

Use `-F` to trace function definitions instead of variables. envtrace detects `function_name() { ... }` definitions, `autoload` declarations (zsh), and `unset -f` removals.

Trace where a function is defined:

```bash
envtrace -F nvm
```

```
nvm() [function]

TRACE (macOS Interactive Login):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[1] ~/.zshrc:23
    nvm() {
    → defines function (15 lines)
       [ -z "$NVM_DIR" ] && return
       \. "$NVM_DIR/nvm.sh"
       nvm "$@"
       ...

DEFINED: yes
```

Trace an autoloaded zsh function:

```bash
envtrace -F compinit
```

```
compinit() [function]

TRACE (macOS Interactive Login):
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[1] /etc/zshrc:5
    autoload -Uz compinit
    → autoloads function (lazy-loaded on first call)

DEFINED: yes
```

Find all files that define a function:

```bash
envtrace -F --find my_function
```

Compare a function across contexts:

```bash
envtrace -F -C login,interactive my_function
```

### JSON output

All modes support `--format json` for scripting and integration with other tools:

```bash
envtrace --format json PATH
envtrace -F --format json nvm
```

```json
{
  "name": "PATH",
  "final_value": "/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin",
  "context": "MacInteractiveLogin",
  "changes": [
    {
      "file": "/etc/zprofile",
      "line_number": 5,
      "line_content": "export PATH=/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin",
      "operation": "Export",
      "value_before": null,
      "value_after": "/usr/local/bin:/usr/bin:/bin:/usr/sbin:/sbin"
    }
  ]
}
```

### System sanity checks

Run `--check` to scan your environment for common issues -- duplicate PATH entries, non-existent directories, and shell/launchd mismatches:

```bash
envtrace --check
```

```
Environment Health Check

PATH Analysis:
────────────────────────────────────────

! Non-existent directories in PATH:
  - /usr/local/go/bin

! Duplicate entries in PATH:
  - /opt/homebrew/bin

────────────────────────────────────────
! 2 issue(s) found.
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
