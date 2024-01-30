use core::fmt;
use std::fmt::format;
use std::{error::Error, fs::File};
use std::io::{self, BufRead, BufReader};

use clap::{App, Arg};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    number_lines: bool,
    number_nonblank_lines: bool,
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
        number_lines: matches.is_present("number_lines"),
        number_nonblank_lines: matches.is_present("number_nonblank")
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
        if config.number_lines {
            let header = format!("{:>6}", i);
            println!("{}\t{}", header, line.unwrap());
            i += 1;
        } else if config.number_nonblank_lines {
            let line = line.unwrap();
            if line.is_empty() {
                println!("");
            } else {
                let header = format!("{:>6}", i);
                println!("{}\t{}", header, line);
                i += 1;
            }
        } else {
            println!("{}", line.unwrap());
        }
    }

    Ok(())

}
