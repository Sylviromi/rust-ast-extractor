mod cache;
mod commands;
mod extractor;
mod project;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "rust-ast-extractor",
    about = "Extract structured data from Rust source files into a JSON cache"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index a Rust file or directory (recursive). Skips unchanged files.
    Index {
        /// Path to a .rs file or directory
        path: PathBuf,
    },
    /// Get JSON summary of a file, or raw source of a specific item.
    ///
    /// Examples:
    ///   get src/lib.rs
    ///   get src/lib.rs::my_function
    ///   get src/lib.rs::fn::my_function
    Get {
        /// File path, optionally with ::item or ::kind::item suffix
        target: String,
    },
    /// List all .rs files in a directory with their module-level doc comments.
    Dir {
        /// Path to a directory
        path: PathBuf,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Index { path } => commands::index::run_index(&path),
        Commands::Get { target } => commands::get::run_get(&target),
        Commands::Dir { path } => commands::dir::run_dir(&path),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
