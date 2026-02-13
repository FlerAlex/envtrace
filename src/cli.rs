use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(name = "envtrace")]
#[command(
    author,
    version,
    about = "Trace where environment variables are defined"
)]
#[command(
    long_about = "Traces where Linux and macOS environment variables are defined and \
    modified through the shell startup sequence. Answers the question: \
    \"Where did this environment variable come from?\""
)]
pub struct Args {
    /// The environment variable (or function name with -F) to trace
    #[arg(value_name = "VARIABLE")]
    pub variable: Option<String>,

    /// Trace a shell function instead of a variable
    #[arg(short = 'F', long = "function")]
    pub function: bool,

    /// Find all files that define the variable (ignores context)
    #[arg(short, long)]
    pub find: bool,

    /// Compare variable across contexts (comma-separated)
    #[arg(short = 'C', long, value_delimiter = ',')]
    pub compare: Option<Vec<String>>,

    /// Simulate a specific shell context
    #[arg(short, long)]
    pub context: Option<ContextArg>,

    /// Run environment sanity checks
    #[arg(long)]
    pub check: bool,

    /// Show verbose output including skipped files
    #[arg(short, long)]
    pub verbose: bool,

    /// Output format
    #[arg(long, default_value = "text")]
    pub format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ContextArg {
    /// Interactive login shell
    Login,
    /// Interactive non-login shell
    Interactive,
    /// Non-interactive shell (cron, scripts)
    Cron,
    /// Systemd service (Linux only)
    Systemd,
    /// Launchd agent - GUI apps (macOS only)
    Launchd,
}

impl Args {
    pub fn validate(&self) -> Result<(), String> {
        // Must provide a variable unless running --check
        if self.variable.is_none() && !self.check {
            return Err("Must provide a variable name or use --check".to_string());
        }

        // Validate variable/function name format
        if let Some(ref name) = self.variable
            && !is_valid_identifier(name)
        {
            return Err(format!(
                "'{}' is not a valid identifier (must match [A-Za-z_][A-Za-z0-9_]*)",
                name
            ));
        }

        // --compare requires a variable
        if self.compare.is_some() && self.variable.is_none() {
            return Err("--compare requires a variable name".to_string());
        }

        // --find requires a variable
        if self.find && self.variable.is_none() {
            return Err("--find requires a variable name".to_string());
        }

        // --function is incompatible with --check
        if self.function && self.check {
            return Err("--function cannot be used with --check".to_string());
        }

        // --function with --find or --compare requires a name
        if self.function && self.find && self.variable.is_none() {
            return Err("--function --find requires a function name".to_string());
        }
        if self.function && self.compare.is_some() && self.variable.is_none() {
            return Err("--function --compare requires a function name".to_string());
        }

        Ok(())
    }
}

fn is_valid_identifier(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}
