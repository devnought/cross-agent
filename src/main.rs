use std::{io::Write, path::PathBuf};

use clap::{Parser, Subcommand};
use humanize_bytes::humanize_bytes_decimal;
use normpath::PathExt;

mod files;
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
        /// Root path
        root: PathBuf,
    },

    /// Start the web service
    Serve {
        /// Web service port
        port: u16,
    },
}

#[derive(Debug, Subcommand)]
enum FilesCommands {
    /// Recursively list all files from a specified root
    List {
        /// Root path
        root: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    match args.command {
        Commands::Files(command) => match command {
            FilesCommands::List { root } => {
                let iter = files::file_iter(root).filter_map(|e| e.path().normalize().ok());
                let mut lock = std::io::stdout().lock();

                for path in iter {
                    writeln!(lock, "{}", path.as_path().display())?;
                }
            }
        },
        Commands::System => sys_info::display(),
        Commands::PathSize { root } => {
            let size = files::file_iter(root)
                .filter_map(|e| e.metadata().ok())
                .map(|m| m.len())
                .sum::<u64>();

            println!("{}", humanize_bytes_decimal!(size));
        }
        Commands::Serve { port } => web_service::start(port).await?,
    }

    Ok(())
}
