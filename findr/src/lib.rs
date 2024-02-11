
use clap::{Parser, ValueEnum};
use regex::Regex;

use walkdir::{DirEntry, WalkDir};
use EntryType::*;
use std::error::Error;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Eq, PartialEq, Clone, ValueEnum)]
enum EntryType {
    Dir,
    File,
    Link,
}

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Config {

    // Search path(s)
    #[arg(value_name = "PATH", default_value = ".")]
    paths: Vec<String>,

    /// Patterns to search
    #[arg(
        short = 'n',
        long = "name",
        value_name = "NAME",
        value_parser(Regex::new),
        num_args(0..)
    )]
    names: Option<Vec<Regex>>,

    /// Entry type to filter result
    #[arg(
        short = 't',
        long = "type",
        value_name = "TYPE",
        value_parser(clap::value_parser!(EntryType)),
        num_args(1..)
    )]
    #[clap(value_enum)]
    entry_types: Option<Vec<EntryType>>,
}

pub fn get_config() -> MyResult<Config> {
    Ok(Config::parse())
}


pub fn run(config: Config) -> MyResult<()> {

    let match_by_type = |entry: & DirEntry| {
        match &config.entry_types {
            None => true,
            Some(entry_types) => {
                entry_types.iter()
                    .any(|entry_type| {
                        match entry_type {
                            File => entry.file_type().is_file(),
                            Dir => entry.file_type().is_dir(),
                            Link => entry.file_type().is_symlink(),
                        }
                    })
            }
        }
    };

    let match_by_name = |entry: & DirEntry| {
        match &config.names {
            None => true,
            Some(names) => {
                let entry_name = entry.file_name().to_string_lossy();
                names.iter()
                    .any(|name| {
                        name.is_match(&entry_name)
                    })
            }
        }
    };

    for path in config.paths {
        for entry in WalkDir::new(path) {
            match entry {
                Err(e) => eprintln!("{}", e),
                Ok(entry) => {
                    if match_by_type(&entry) && match_by_name(&entry) {
                        println!("{}", entry.path().display())
                    }
                }
            }
        }
    }

    Ok(())
}

