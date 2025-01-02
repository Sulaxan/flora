use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct FloraCli {
    /// Command to run
    #[command(subcommand)]
    pub command: FloraSubcommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum FloraSubcommand {
    /// Start the widget daemon
    Start {
        /// The path to the config
        #[arg(short, long)]
        config_path: PathBuf,
    },
}
