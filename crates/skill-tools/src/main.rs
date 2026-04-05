use alan_skill_tools::{generate_review_bundle, regenerate_benchmark};
use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "alan-skill-tools",
    about = "Reusable authoring and eval tooling for Alan skill packages"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Recompute benchmark.json from an existing eval run directory
    AggregateBenchmark {
        /// Eval run directory containing run.json
        run_dir: PathBuf,
    },
    /// Rebuild the static review bundle for an existing eval run directory
    GenerateReview {
        /// Eval run directory containing run.json
        run_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::AggregateBenchmark { run_dir } => {
            let path = regenerate_benchmark(&run_dir)?;
            println!("{}", path.display());
        }
        Command::GenerateReview { run_dir } => {
            let path = generate_review_bundle(&run_dir)?;
            println!("{}", path.display());
        }
    }
    Ok(())
}
