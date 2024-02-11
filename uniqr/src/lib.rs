use clap::Parser;
use std::{error::Error, fs::File, io::{self, BufRead, BufReader, BufWriter, Write}};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Config {
    /// Input file
    #[arg(value_name = "IN_FILE", default_value = "-")]
    in_file: String,

    /// Output file
    #[arg(value_name = "OUT_FILE")]
    out_file: Option<String>,

    /// Show counts
    #[arg(short, long)]
    count: bool,
}

pub fn get_args() -> MyResult<Config> {
    Ok(Config::parse())
}

fn open_in(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?)))
    }
}

fn open_out(filename: &Option<String>) -> MyResult<Box<dyn Write>> {
    match filename {
        None => Ok(Box::new(BufWriter::new(io::stdout()))),
        Some(filename) => Ok(Box::new(BufWriter::new(File::create(filename)?)))
    }
}

fn print_line(count_flg: bool, count: usize, line: &String, file_out: &mut Box<dyn Write>) -> MyResult<()> {
    if count_flg {
        write!(file_out, "{:>4} {}", count, line)?;
    } else {
        write!(file_out, "{}", line)?;
    }   

    Ok(())
}

pub fn run(config: Config) -> MyResult<()> {
    let mut file_in = open_in(&config.in_file)
        .map_err(|e| format!("{}: {}", config.in_file, e))?;

    let mut file_out = open_out(&config.out_file)
        .map_err(|e| format!("{}: {}", config.out_file.unwrap_or("stdout".to_string()), e))?;

    let mut count: usize = 0;
    let mut prev_line = String::new();

    loop {
        let mut line = String::new();
        let bytes = file_in.read_line(&mut line)?;
        if bytes == 0 {
            if count > 0 { print_line(config.count, count, &prev_line, &mut file_out)? }
            break;
        }

        match (prev_line.trim_matches('\n') == line.trim_matches('\n'), count) {
            (_, 0) => {
                count += 1;
                prev_line = line;
            }
            (true, _) => {
                // 最終行とその前の行の違いが改行の有無しかない場合、それは同じものとして処理するので
                // その場合にここで prev_line = line をしてしまうと
                // prev_line を改行がないもので上書きしてしまう
                count += 1;
            }
            (false, _) => {
                print_line(config.count, count, &prev_line, &mut file_out)?;
                count = 1;
                prev_line = line;
            }
        }
    }

    Ok(())
}




