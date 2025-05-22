use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about)]
pub struct Cli {
    #[arg(default_value = ".")]
    pub path: PathBuf,

    #[command(subcommand)]
    pub command: Commands,

    #[arg(long)]
    pub max_size: Option<u64>,

    #[arg(long)]
    pub skip_binary: bool,

    #[arg(long)]
    pub dry_run: bool,

    #[arg(long)]
    pub log: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    Search {
        pattern: String,

        #[arg(value_enum, default_value_t = SearchMode::Regex)]
        mode: SearchMode,

        #[arg(long)]
        copy: bool,

        #[arg(long, short, value_delimiter = ',')]
        extensions: Option<Vec<String>>,
    },
    List {
        #[arg(long, short, value_delimiter = ',')]
        extensions: Option<Vec<String>>,
        
        #[arg(long, short)]
        long: bool,
        
        #[arg(long)]
        recursive: bool,
        
        #[arg(long)]
        show_hidden: bool,
    },
}

#[derive(ValueEnum, Clone, Debug)]
pub enum SearchMode {
    Regex,
    Text,
    Word,
}