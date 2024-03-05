mod args;

use clap::Parser;
use color_eyre::eyre::ContextCompat;
use color_eyre::{eyre::WrapErr, Report, Result};
use rayon::prelude::*;
use std::io::BufRead;
use std::path::{Path, PathBuf};

fn main() -> Result<()> {
    color_eyre::install()?;

    let args = args::Args::parse();

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
                    glob(path, &regex, args.regex_not)
                } else {
                    glob_recursive(path, &regex, args.regex_not)
                }?;

                files
                    .par_iter()
                    .try_fold(
                        || 0,
                        |acc, path| {
                            std::fs::File::open(path)
                                .map(|file| acc + std::io::BufReader::new(file).lines().count())
                                .wrap_err(format!("Failed to read file {path:?}"))
                        },
                    )
                    .try_reduce(
                        || 0,
                        |a, b| usize::checked_add(a, b).wrap_err("Line count overflowed"),
                    )
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

fn glob(path: &Path, regex: &regex::Regex, regex_not: bool) -> Result<Vec<PathBuf>> {
    let mut result = Vec::new();

    for dir_entry in std::fs::read_dir(path)? {
        let dir_entry = dir_entry.wrap_err("Failed to read directory")?;
        let metadata = dir_entry.metadata()?;

        if metadata.is_file()
            && regex_not
                != regex.is_match(
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

fn glob_recursive(path: &Path, regex: &regex::Regex, regex_not: bool) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut directories = Vec::new();

    for dir_entry in path
        .read_dir()
        .wrap_err_with(|| format!("Failed to read directory {path:?}"))?
    {
        let dir_entry = dir_entry?;
        let metadata = dir_entry.metadata()?;

        if metadata.is_file() {
            if regex_not
                != regex.is_match(
                    dir_entry
                        .file_name()
                        .to_str()
                        .wrap_err(FILE_NAME_REGEX_STR_CONV_FAIL)?,
                )
            {
                files.push(dir_entry.path());
            }
        } else if metadata.is_dir() {
            directories.push(dir_entry.path());
        } else if regex_not
            != regex.is_match(
                dir_entry
                    .file_name()
                    .to_str()
                    .wrap_err(FILE_NAME_REGEX_STR_CONV_FAIL)?,
            )
        {
            return Err(Report::msg(format!(
                "{:?} is neither file or directory",
                dir_entry.path()
            )));
        }
    }

    files.append(
        &mut directories
            .par_iter()
            .flat_map(|path| match glob_recursive(path, regex, regex_not) {
                Ok(paths) => paths.into_iter().map(Ok).collect(),
                Err(e) => {
                    vec![Err(e)]
                }
            })
            .collect::<Result<Vec<PathBuf>>>()?,
    );

    Ok(files)
}
