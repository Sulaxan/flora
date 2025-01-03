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
    /// List all flora processes
    List,
    /// Show specific or all widgets
    Show {
        /// Show all widgets
        #[arg(long, action)]
        all: bool,
        /// The specific widget to show
        name: Option<String>,
    },
    /// Hide specific or all widgets
    Hide {
        /// Hide all widgets
        #[arg(long, action)]
        all: bool,
        /// The specific widget to hide
        name: Option<String>,
    },
}
