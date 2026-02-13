use clap::Parser;

use envtrace::cli::{Args, ContextArg, OutputFormat};
use envtrace::output::{
    compare_function, compare_variable, format_function_trace, format_function_trace_json,
    format_trace, format_trace_json, run_checks,
};
use envtrace::platform::Platform;
use envtrace::trace::{Context, TraceConfig, TraceEngine};

fn main() {
    let args = Args::parse();

    if let Err(e) = args.validate() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    let platform = Platform::detect();

    // Set up the engine
    let config = TraceConfig {
        follow_sources: true,
        verbose: args.verbose,
    };
    let mut engine = TraceEngine::new(platform).with_config(config);

    if args.check {
        print!("{}", run_checks(platform, args.verbose));
        return;
    }

    let var_name = args.variable.as_ref().unwrap();

    // Determine context
    let context = match args.context {
        Some(ctx) => context_from_arg(ctx, platform),
        None => Context::default_for_platform(),
    };

    if args.function {
        // Function tracing mode
        if args.find {
            let changes = engine.find_all_functions(var_name);

            if changes.is_empty() {
                println!("No definitions of {}() found in config files.", var_name);
            } else {
                let home_prefix = dirs::home_dir()
                    .map(|h| h.to_string_lossy().to_string())
                    .unwrap_or_default();
                println!("Found {} definition(s) of {}():\n", changes.len(), var_name);
                for change in changes {
                    let file_display = change.file.to_string_lossy().replace(&home_prefix, "~");
                    println!("  {}:{}", file_display, change.line_number);
                    println!("    {}", change.line_content);
                    if let Some(ref body) = change.body {
                        for line in body.lines().take(3) {
                            println!("    {}", line);
                        }
                    }
                    println!();
                }
            }
        } else if let Some(ref contexts) = args.compare {
            print!(
                "{}",
                compare_function(&mut engine, var_name, contexts, platform)
            );
        } else {
            let trace = engine.trace_function(var_name, context);

            match args.format {
                OutputFormat::Text => {
                    print!("{}", format_function_trace(&trace));
                }
                OutputFormat::Json => {
                    println!("{}", format_function_trace_json(&trace));
                }
            }
        }
    } else if args.find {
        // --find mode: show all definitions regardless of context
        let changes = engine.find_all(var_name);

        if changes.is_empty() {
            println!("No definitions of {} found in config files.", var_name);
        } else {
            let home_prefix = dirs::home_dir()
                .map(|h| h.to_string_lossy().to_string())
                .unwrap_or_default();
            println!("Found {} definition(s) of {}:\n", changes.len(), var_name);
            for change in changes {
                let file_display = change.file.to_string_lossy().replace(&home_prefix, "~");
                println!("  {}:{}", file_display, change.line_number);
                println!("    {}", change.line_content);
                println!();
            }
        }
    } else if let Some(ref contexts) = args.compare {
        // --compare mode: show variable across multiple contexts
        print!(
            "{}",
            compare_variable(&mut engine, var_name, contexts, platform)
        );
    } else {
        // Standard trace mode
        let trace = engine.trace(var_name, context);

        match args.format {
            OutputFormat::Text => {
                print!("{}", format_trace(&trace));
            }
            OutputFormat::Json => {
                println!("{}", format_trace_json(&trace));
            }
        }
    }
}

/// Convert CLI context argument to internal Context enum
fn context_from_arg(arg: ContextArg, platform: Platform) -> Context {
    match (arg, platform) {
        (ContextArg::Login, Platform::MacOS) => Context::MacInteractiveLogin,
        (ContextArg::Login, Platform::Linux) => Context::InteractiveLogin,
        (ContextArg::Interactive, Platform::MacOS) => Context::MacInteractiveNonLogin,
        (ContextArg::Interactive, Platform::Linux) => Context::InteractiveNonLogin,
        (ContextArg::Cron, _) => Context::NonInteractiveNonLogin,
        (ContextArg::Systemd, _) => Context::SystemdService,
        (ContextArg::Launchd, _) => Context::LaunchdAgent,
    }
}
