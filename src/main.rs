mod atcoder;
mod builder;
mod commands;
mod config;
mod fs_layout;
mod models;
mod project_config;
mod run_test;
mod status;
mod template_gen;
mod ui;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(name = "acx")]
#[command(about = "CLI tool for AtCoder", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Download statements and test cases for a contest
    Download {
        /// Contest ID, e.g. abc300
        contest: String,
        /// Number of parallel download jobs
        #[arg(short, long, default_value_t = 2)]
        jobs: usize,
    },
    /// Run test cases against a solution command
    Test {
        /// Problem selector: none, problem ID, or contest ID + problem ID
        args: Vec<String>,
        /// Build profile from acx.json
        #[arg(short, long)]
        profile: Option<String>,
        /// Case number to run. Can be specified multiple times
        #[arg(short, long = "case")]
        case: Vec<usize>,
    },
    /// Show submission status for a contest
    Status {
        /// Contest ID. Inferred from current directory when omitted
        contest: Option<String>,
        /// Output mode
        #[arg(long, value_enum, default_value_t = StatusMode::Summary)]
        mode: StatusMode,
        /// Filter rows
        #[arg(long, value_enum, default_value_t = StatusFilter::All)]
        only: StatusFilter,
        /// Number of submissions to show in list mode
        #[arg(long, default_value_t = 20)]
        tail: usize,
        /// Maximum number of submission pages to fetch
        #[arg(long, default_value_t = 20)]
        max_pages: usize,
    },
    /// Open statement and source with the configured editor command
    Solve {
        /// Problem selector: none, problem ID, or contest ID + problem ID
        args: Vec<String>,
    },
    /// Generate a C++ solution template
    Template {
        /// Problem selector: none, problem ID, or contest ID + problem ID
        args: Vec<String>,
        /// Overwrite an existing template file
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum StatusMode {
    Summary,
    List,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum StatusFilter {
    All,
    Failed,
    Unsolved,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Download { contest, jobs } => commands::download(&contest, jobs),
        Command::Test {
            args,
            profile,
            case,
        } => commands::test(&args, profile.as_deref(), &case),
        Command::Status {
            contest,
            mode,
            only,
            tail,
            max_pages,
        } => commands::status(
            contest.as_deref(),
            commands::StatusOptions {
                mode: match mode {
                    StatusMode::Summary => commands::StatusMode::Summary,
                    StatusMode::List => commands::StatusMode::List,
                },
                only: match only {
                    StatusFilter::All => commands::StatusFilter::All,
                    StatusFilter::Failed => commands::StatusFilter::Failed,
                    StatusFilter::Unsolved => commands::StatusFilter::Unsolved,
                },
                tail,
                max_pages,
            },
        ),
        Command::Solve { args } => commands::solve(&args),
        Command::Template { args, force } => commands::template(&args, force),
    }
}
