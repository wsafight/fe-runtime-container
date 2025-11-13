mod config;
mod manager;
mod project;
mod runtime;
mod storage;

use anyhow::Result;
use clap::{Parser, Subcommand};
use manager::Manager;
use runtime::Runtime;

#[derive(Parser)]
#[command(name = "frc")]
#[command(version)]
#[command(about = "Frontend Runtime Container - Manage JS runtime memory settings", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Runtime command (node, deno, bun, npm, npx, pnpm, yarn, etc.)
    #[arg(value_name = "COMMAND")]
    runtime_cmd: Option<String>,

    /// Arguments to pass to the runtime
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,

    /// Memory limit in MB (e.g., 4096 for 4GB)
    /// When specified, it will be saved for this project
    #[arg(short, long)]
    memory: Option<String>,

    /// Explicitly specify runtime (node, deno, bun)
    /// Useful for commands where runtime cannot be auto-detected
    #[arg(short, long, value_name = "RUNTIME")]
    runtime: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Show memory recommendations for a runtime
    Info {
        /// Runtime (node, deno, bun)
        runtime: String,
    },

    /// Show current project's saved configuration
    Project,

    /// List all saved project configurations
    #[command(name = "list")]
    ListProjects,

    /// Remove saved configuration for current or specified project
    Forget {
        /// Optional project path (uses current directory if not specified)
        path: Option<String>,
    },

    /// Clean up old project configurations
    Cleanup {
        /// Remove configs older than this many days (default: 30)
        #[arg(short, long, default_value = "30")]
        days: u64,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Info { runtime }) => {
            let rt = Runtime::from_command(&runtime)?;
            let manager = Manager::new()?;
            manager.show_recommendations(&rt)?;
        }
        Some(Commands::Project) => {
            let manager = Manager::new()?;
            manager.show_project()?;
        }
        Some(Commands::ListProjects) => {
            let manager = Manager::new()?;
            manager.list_projects()?;
        }
        Some(Commands::Forget { path }) => {
            let mut manager = Manager::new()?;
            manager.forget_project(path)?;
        }
        Some(Commands::Cleanup { days }) => {
            let mut manager = Manager::new()?;
            manager.cleanup(days)?;
        }
        None => {
            // Direct command execution
            if let Some(cmd) = cli.runtime_cmd {
                // Detect runtime: use explicit runtime flag or auto-detect from command
                let runtime_specified = cli.runtime.is_some();
                let runtime = if let Some(rt) = &cli.runtime {
                    Runtime::from_command(rt)?
                } else {
                    Runtime::from_command(&cmd)?
                };

                let mut manager = Manager::new()?;

                // Prepare arguments - include the original command if it's not the base runtime
                let mut exec_args = Vec::new();

                // If runtime was explicitly specified, include the full command
                if runtime_specified {
                    exec_args.push(cmd);
                } else if cmd != runtime.as_str() {
                    // For commands like npm, npx, vite, etc.
                    exec_args.push(cmd);
                }

                exec_args.extend(cli.args);

                // If memory is explicitly provided, save it to project config
                let save_to_project = cli.memory.is_some();

                manager.run(&runtime, &exec_args, cli.memory, save_to_project)?;
            } else {
                print_usage();
            }
        }
    }

    Ok(())
}

fn print_usage() {
    println!("frc - Frontend Runtime Container");
    println!();
    println!("USAGE:");
    println!("  frc [OPTIONS] <COMMAND> [ARGS]...");
    println!();
    println!("OPTIONS:");
    println!("  -m, --memory <MB>       Set memory limit in MB (saves to project config)");
    println!("  -r, --runtime <RUNTIME> Specify runtime (node/deno/bun) explicitly");
    println!("  -h, --help              Show help information");
    println!("  -V, --version           Show version");
    println!();
    println!("COMMANDS:");
    println!("  info <runtime>       Show memory recommendations");
    println!("  project              Show current project's saved config");
    println!("  list                 List all saved project configs");
    println!("  forget [path]        Remove saved config for project");
    println!("  cleanup --days <N>   Remove configs older than N days");
    println!();
    println!("EXAMPLES:");
    println!("  # First time in a project - saves 4GB config");
    println!("  frc -m 4096 node index.js");
    println!();
    println!("  # Later runs - uses saved 4GB automatically");
    println!("  frc node index.js");
    println!();
    println!("  # Explicitly specify runtime for unknown commands");
    println!("  frc -r node -m 4096 my-custom-script");
    println!("  frc --runtime deno tsx build.ts");
    println!();
    println!("  # View current project config");
    println!("  frc project");
    println!();
    println!("  # List all saved projects");
    println!("  frc list");
    println!();
    println!("  # Remove saved config");
    println!("  frc forget");
    println!();
    println!("SUPPORTED RUNTIMES:");
    println!("  Node.js: node, npm, npx, pnpm, yarn    [Memory config: ✓]");
    println!("  Deno:    deno                          [Memory config: ✓]");
    println!("  Bun:     bun                           [Memory config: ✗]");
    println!();
    println!("HOW IT WORKS:");
    println!("  1. When you run with -m flag, the memory config is saved for this project");
    println!("  2. Future runs without -m will use the saved config automatically");
    println!("  3. If no saved config exists, you'll see recommended values");
    println!("  4. Configs are project-specific (detected via package.json, .git, etc.)");
    println!();
    println!("NOTE: Bun uses JavaScriptCore and manages memory automatically.");
}
