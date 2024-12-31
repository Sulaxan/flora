use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// The path to the config
    #[arg(short, long)]
    pub config_path: PathBuf,
}
