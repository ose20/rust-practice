use core::fmt;
use std::{error::Error, fs::File};
use std::io::{self, BufRead, BufReader};

use clap::{Parser, ValueEnum};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Parser, Debug)]
pub struct Config {
    #[arg(help = "Input files")]
    files: Vec<String>,

    #[arg(
        short = 'm',
        long = "mode",
        value_name = "MODE",
        default_value_t = PrintMode::Normal,
    )]
    #[clap(value_enum)]
    print_mode: PrintMode,
}

#[derive(ValueEnum, Clone, Debug, Eq, PartialEq)]
enum PrintMode {
    Normal,
    Number,
    NumberAndNonblank,
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
    Ok(Config::parse())
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
            PrintMode::Number => {
                let header = format!("{:>6}", i);
                println!("{}\t{}", header, line);
                i += 1;
            }
            PrintMode::NumberAndNonblank => {
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
