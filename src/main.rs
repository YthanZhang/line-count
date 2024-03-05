use clap::Parser;
use color_eyre::eyre::ContextCompat;
use color_eyre::{eyre::WrapErr, Report, Result};
use std::io::BufRead;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = Args::parse();

    let line_count = args
        .paths
        .iter()
        .map(|path| {
            if path.is_file() {
                std::fs::File::open(path)
                    .map(|file| std::io::BufReader::new(file).lines().count())
                    .wrap_err_with(|| format!("Failed to open \"{path:?}\""))
            } else {
                let regex = regex::Regex::new(&args.regex_string).wrap_err_with(|| {
                    format!("\"{}\" is not a valid regex pattern", args.regex_string)
                })?;

                let files = if args.no_recurse {
                    glob(path, &regex)
                } else {
                    glob_recursive(path, &regex)
                }?;

                files
                    .iter()
                    .try_fold(0, |acc, path| {
                        std::fs::File::open(path)
                            .map(|file| acc + std::io::BufReader::new(file).lines().count())
                    })
                    .wrap_err_with(|| format!("Failed to open {path:?}"))
            }
        })
        .sum::<Result<usize>>()
        .wrap_err("Failed to count lines")?;

    println!("{line_count}");

    Ok(())
}

const FILE_NAME_REGEX_STR_CONV_FAIL: &str =
    "Failed to convert file name to UTF-8 string, cannot complete regex match";

fn glob(path: &Path, regex: &regex::Regex) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();

    for dir_entry in std::fs::read_dir(path)? {
        let dir_entry = dir_entry.wrap_err("Failed to read directory")?;
        let metadata = dir_entry.metadata()?;

        if metadata.is_file()
            && regex.is_match(
                dir_entry
                    .file_name()
                    .to_str()
                    .wrap_err(FILE_NAME_REGEX_STR_CONV_FAIL)?,
            )
        {
            result.push(dir_entry.path())
        }
    }

    Ok(result)
}

fn glob_recursive(path: &Path, regex: &regex::Regex) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    for dir_entry in path
        .read_dir()
        .wrap_err_with(|| format!("Failed to read directory {path:?}"))?
    {
        let dir_entry = dir_entry?;
        let metadata = dir_entry.metadata()?;

        if metadata.is_file() {
            if regex.is_match(
                dir_entry
                    .file_name()
                    .to_str()
                    .wrap_err(FILE_NAME_REGEX_STR_CONV_FAIL)?,
            ) {
                result.push(dir_entry.path());
            }
        } else if metadata.is_dir() {
            result.append(&mut glob_recursive(&dir_entry.path(), regex)?);
        } else {
            return Err(Report::msg(
                "Found object in directory that is neither file or directory",
            ));
        }
    }

    Ok(result)
}

/// A single file line counter program
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Directories to search for files
    #[arg(num_args = 1..)]
    paths: Vec<PathBuf>,

    /// Set for non-recursive search
    #[arg(long, action)]
    no_recurse: bool,

    /// File name pattern to match, regex expression. Default match all.
    /// Match file extension: `(\.txt)$` matches `*.txt` files.
    /// Match multiple file extensions: `(\.(txt|html))$` matches `*.txt` and `*.html` files.
    #[arg(short = 'r', long = "regex", default_value = ".*")]
    regex_string: String,
}
