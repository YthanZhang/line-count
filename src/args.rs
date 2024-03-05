use std::path::PathBuf;
use clap::Parser;

/// A single file line counter program
#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// Directories to search for files
    #[arg(num_args = 1..)]
    pub paths: Vec<PathBuf>,

    /// Set for non-recursive search
    #[arg(long, action)]
    pub no_recurse: bool,

    /// File name pattern to match, regex expression. Default match all.
    /// Match file extension: `(\.txt)$` matches `*.txt` files.
    /// Match multiple file extensions: `(\.(txt|html))$` matches `*.txt` and `*.html` files.
    #[arg(short = 'r', long = "regex", default_value = ".*")]
    pub regex_string: String,

    #[arg(long, action)]
    /// Filter **OUT** matches when set to true, default false.
    pub regex_not: bool,
}
