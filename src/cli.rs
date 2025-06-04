use clap::{Parser, Subcommand, ValueEnum};
use std::fmt;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(default_value = ".")]
    pub path: PathBuf,

    #[clap(long, value_parser, default_value_t = false)]
    pub verbose: bool,

    #[clap(long, value_parser)]
    pub log: Option<PathBuf>,

    #[clap(long, value_parser, default_value_t = false)]
    pub dry_run: bool,

    #[clap(long, value_parser)]
    pub max_size: Option<usize>,

    #[clap(long, value_parser, default_value_t = false)]
    pub skip_binary: bool,

    #[clap(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Search {
        pattern: String,

        #[clap(value_parser, default_value_t = SearchMode::Text)]
        mode: SearchMode,

        #[clap(long, value_parser, default_value_t = false)]
        copy: bool,

        #[clap(long, value_parser, use_value_delimiter = true)]
        extensions: Option<Vec<String>>,

        #[clap(short, long, value_parser, default_value_t = false)]
        recursive: bool,
    },
    List {
        #[clap(long, value_parser, use_value_delimiter = true)]
        extensions: Option<Vec<String>>,
        #[clap(short, long, value_parser, default_value_t = false)]
        long: bool,
        #[clap(short, long, value_parser, default_value_t = false)]
        recursive: bool,
        #[clap(long, value_parser, default_value_t = false)]
        show_hidden: bool,
    },
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum SearchMode {
    #[default]
    Text,
    Word,
    Regex,
}

impl fmt::Display for SearchMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchMode::Text => write!(f, "text"),
            SearchMode::Word => write!(f, "word"),
            SearchMode::Regex => write!(f, "regex"),
        }
    }
}
