use core::fmt;
use std::{error::Error, fs::File};
use std::io::{self, BufRead, BufReader};

use clap::{Parser, ValueEnum};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Parser, Debug)]
#[command(about = "something like cat command")]
pub struct Arg {
    #[arg(help = "The files to output (default is stdin if not specified)")]
    files: Option<Vec<String>>,

    /// Output format
    #[arg(
        short = 'm',
        long = "mode",
        value_name = "MODE",
        default_value_t = PrintMode::Normal,
    )]
    #[clap(value_enum)]
    print_mode: PrintMode,
}

impl Arg {
    // parse した arg を config に変換する
    fn to_config(self) -> Config {
        Config {
            input: {
                match self.files {
                    None => Input::Stdin,
                    Some(files) => Input::Files(files),
                }
            },
            print_mode: self.print_mode,
        }
    }
}


#[derive(ValueEnum, Clone, Debug, Eq, PartialEq)]
enum PrintMode {
    Normal,
    Number,
    NumberAndNonblank,
}

pub struct Config {
    input: Input,

    print_mode: PrintMode,
}

enum Input {
    Stdin,
    Files(Vec<String>)
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

pub fn get_config() -> MyResult<Config> {
    Ok(Arg::parse().to_config())
}

// None なら stdin、 Some(file) なら file への buf_reader を返す
fn open(input: Option<&str>) -> MyResult<Box<dyn BufRead>> {
    match input {
        None => Ok(Box::new(BufReader::new(io::stdin()))),
        Some(filename) => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

pub fn run(config: Config) -> MyResult<()> {
    let mut err_flg = false;

    match &config.input {
        Input::Stdin => {
            match open(None) {
                Err(err) => {
                    eprintln!("Failed to open stdin: {}", err);
                    err_flg = true;
                },
                Ok(buf_reader) => cat_file(&config, buf_reader)?
            }
        }
        Input::Files(files) => {
            for filename in files {
                match open(Some(filename)) {
                    Err(err) => {
                        eprintln!("Failed to open {}: {}", filename, err);
                        err_flg = true;
                    },
                    Ok(buf_reader) => {
                        cat_file(&config, buf_reader)?
                    }
                }
            }

        }
    }


    if err_flg {
        Err(Box::new(io::Error::new(io::ErrorKind::Other, "少なくとも一つのファイルでエラーがありました")))
        // Err(From::from("少なくとも1つのファイルでエラーがありました")) ← こっちの方が簡潔だけど、自分でエラーを定義する例として残したいので変えない
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
