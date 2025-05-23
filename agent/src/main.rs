use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod stream;
mod sys_info;
mod web_service;

/// Get data from the system
#[derive(Debug, Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// File operations
    #[command(subcommand)]
    Files(FilesCommands),

    /// System information
    System,

    /// Total size of all files from root path
    PathSize {
        /// Root file paths
        #[clap(required = true)]
        root: Vec<PathBuf>,
    },

    /// Start the web service
    Serve {
        /// Web service port
        port: u16,

        /// Root file paths
        #[clap(required = true)]
        root: Vec<PathBuf>,
    },
}

#[derive(Debug, Subcommand)]
enum FilesCommands {
    /// Recursively list all files from a specified root
    List {
        /// Root file paths
        #[clap(required = true)]
        root: Vec<PathBuf>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();

    simplelog::CombinedLogger::init(vec![simplelog::TermLogger::new(
        log::LevelFilter::Trace,
        simplelog::Config::default(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )])?;

    match args.command {
        Commands::Files(command) => match command {
            FilesCommands::List { root: _ } => {
                // let iter = files::file_iter(root.iter(), paths.iter())
                //     .filter_map(|e| e.path().normalize().ok());
                // let mut lock = std::io::stdout().lock();

                // for path in iter {
                //     writeln!(lock, "{}", path.as_path().display())?;
                // }
            }
        },
        Commands::System => sys_info::display(),
        Commands::PathSize { root: _ } => {
            // let size = files::file_iter(root.iter(), paths.iter())
            //     .filter_map(|e| e.metadata().ok())
            //     .map(|m| m.len())
            //     .sum::<u64>();

            // println!("{}", humanize_bytes_decimal!(size));
        }
        Commands::Serve { port, root } => web_service::start(port, root).await?,
    }

    Ok(())
}
