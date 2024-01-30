use core::fmt;
use std::{error::Error, fs::File};
use std::io::{self, BufRead, BufReader};

use clap::{App, Arg};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
enum PrintMode {
    Normal,
    PrintAll,
    PrintNonblank,
}

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    print_mode: PrintMode,
}

#[derive(Debug)]
struct FileOpenError {
    filename: String,
    source: io::Error,
}

impl fmt::Display for FileOpenError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Failed to open {}: {}", self.filename, self.source)
    }
}

impl Error for FileOpenError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.source)
    }
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("catr")
        .version("0.1.0")
        .author("ose20 <ose20dive@gmail.com>")
        .about("Rust cat")
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .help("Input file(s)")
                .multiple(true)
                .default_value("-"),
        )
        .arg(
            Arg::with_name("number_lines")
                .short("n")
                .long("number")
                .help("number all output lines")
                .takes_value(false)
                .conflicts_with("number_nonblank"),
        )
        .arg(
            Arg::with_name("number_nonblank")
                .short("b")
                .long("number-nonblank")
                .help("number nonempty output lines")
                .takes_value(false)
        )
        .get_matches();

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        print_mode:
            if matches.is_present("number_lines") {
                PrintMode::PrintAll
            } else if matches.is_present("number_nonblank") {
                PrintMode::PrintNonblank
            } else {
                PrintMode::Normal
            }
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

pub fn run(config: Config) -> MyResult<()> {
    let mut err_flg = false;

    for filename in &config.files {
        match open(filename) {
            Err(err) => {
                eprintln!("Failed to open {}: {}", filename, err);
                err_flg = true;
            },
            Ok(bufreader) => cat_file(&config, bufreader)?,
        }
    }

    if err_flg { 
        Err(Box::new(io::Error::new(io::ErrorKind::Other, "少なくとも一つのファイルでエラーがありました")))
    } else {
        Ok(())
    }
}


fn cat_file(config: &Config, bufreader: Box<dyn BufRead>) -> MyResult<()> {
    let mut i = 1;
    for line in bufreader.lines() {
        let line = line.unwrap();
        match config.print_mode {
            PrintMode::Normal => {
                println!("{}", line);
            }
            PrintMode::PrintAll => {
                let header = format!("{:>6}", i);
                println!("{}\t{}", header, line);
                i += 1;
            }
            PrintMode::PrintNonblank => {
                if line.is_empty() {
                    println!("");
                } else {
                    let header = format!("{:>6}", i);
                    println!("{}\t{}", header, line);
                    i += 1;
                }
            }
        }
    }

    Ok(())

}
