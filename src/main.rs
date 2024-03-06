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

    println!("{}", get_line_count(&args)?);

    Ok(())
}

fn get_line_count(args: &args::Args) -> Result<usize> {
    args.paths
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
                    glob_recursive(path.to_owned(), &regex, args.regex_not)
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
        .wrap_err("Failed to count lines")
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

fn glob_recursive(path: PathBuf, regex: &regex::Regex, regex_not: bool) -> Result<Vec<PathBuf>> {
    fn glob_split(
        path: &Path,
        regex: &regex::Regex,
        regex_not: bool,
    ) -> Result<(Vec<PathBuf>, Vec<PathBuf>)> {
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

        Ok((files, directories))
    }

    let mut directories = vec![vec![path]];
    let mut files = vec![];

    while !directories.is_empty() {
        let (new_files, dirs): (Vec<Vec<PathBuf>>, Vec<Vec<PathBuf>>) = directories
            .into_par_iter()
            .flatten()
            .map(|path| glob_split(&path, regex, regex_not))
            .collect::<Result<_>>()?;

        files.extend(new_files.into_iter().flatten());
        directories = dirs;
    }

    Ok(files)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_glob() {
        let mut args = args::Args {
            paths: vec![PathBuf::from("./test")],
            no_recurse: true,
            regex_string: ".*".to_string(),
            regex_not: false,
        };

        assert_eq!(get_line_count(&args).unwrap(), 532);

        args.paths = vec![
            PathBuf::from("./test"),
            PathBuf::from("./test/.a/Cargo.lock"),
            PathBuf::from("./test/b/"),
        ];
        args.regex_string = "(\\.lock)$".to_string();
        assert_eq!(get_line_count(&args).unwrap(), 532 * 2);

        // only globed files get regex check, file path doesn't
        args.regex_not = true;
        assert_eq!(get_line_count(&args).unwrap(), 532 + 1 + 14);
    }

    #[test]
    fn test_glob_recurse() {
        let mut args = args::Args {
            paths: vec![PathBuf::from("./test")],
            no_recurse: false,
            regex_string: ".*".to_string(),
            regex_not: false,
        };
        assert_eq!(get_line_count(&args).unwrap(), 532 * 2 + 1 + 14);

        args.regex_string = "(\\.lock)$".to_string();
        assert_eq!(get_line_count(&args).unwrap(), 532 * 2);

        args.regex_not = true;
        assert_eq!(get_line_count(&args).unwrap(), 1 + 14);
    }
}
