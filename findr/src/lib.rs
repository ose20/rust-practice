use clap::{App, Arg};
use regex::Regex;

use walkdir::{DirEntry, WalkDir};
use EntryType::*;
use std::error::Error;


type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Eq, PartialEq)]
enum EntryType {
    Dir,
    File,
    Link,
}

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    names: Option<Vec<Regex>>,
    entry_types: Option<Vec<EntryType>>,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("findr")
        .version("0.1.0")
        .author("ose20 <ose20@gmail.com>")
        .about("Rust find")
        .arg(
            Arg::with_name("paths")
            .value_name("PATH")
            .help("Search paths")
            .default_value(".")
            .multiple(true)
        )
        .arg(
            Arg::with_name("names")
            .value_name("NAME")
            .short("n")
            .long("name")
            .help("Name")
            .multiple(true),
        )
        .arg(
            Arg::with_name("types")
            .value_name("TYPE")
            .short("t")
            .long("type")
            .help("Entry type")
            .possible_values(&["f", "d", "l"])
            .multiple(true),
        )
        .get_matches();

    let names = matches
        .values_of_lossy("names")
        .map(|vals| {
            vals.into_iter()
                .map(|name| {
                    Regex::new(&name)
                        .map_err(|_| format!("Invalid --name \"{}\"", name))
                })
                .collect()
        })
        .transpose()?;

    let entry_types = matches
        .values_of_lossy("types")
        .map(|vals| {
            vals.iter()
                .map(|val| match val.as_str() {
                    "d" => Dir,
                    "f" => File,
                    "l" => Link,
                    _ => unreachable!("Invalid type"),
                })
                .collect()
        });

    Ok(Config {
        paths: matches.values_of_lossy("paths").unwrap(),
        names,
        entry_types,
    })
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

