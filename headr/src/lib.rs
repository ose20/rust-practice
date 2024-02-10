use std::{error::Error, fs::File, io::{self, BufRead, BufReader, Read}};
use clap::Parser;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Input file(s) (stdin if not specified)
    #[arg(value_name = "FILE")]
    files: Option<Vec<String>>,

    /// Number of lines
    #[arg(
        short('n'),
        long,
        default_value = "10",
        value_name = "LINES",
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    lines: u64,

    /// Number of bytes
    #[arg(
        short('c'),
        long,
        value_name = "BYTES",
        conflicts_with("lines"),
        value_parser = clap::value_parser!(u64).range(1..)
    )]
    bytes: Option<u64>,
}

impl Args {
    fn to_config(self) -> MyResult<Config> {
        Ok(Config{
            files: self.files,
            print_mode: {
                if let Some(byte_size) = self.bytes { PrintMode::ByteMode(byte_size as usize) }
                else { PrintMode::LineMode(self.lines as usize) }
            }
        })
    }
}

#[derive(Debug)]
enum PrintMode {
    LineMode(usize),
    ByteMode(usize),
}

#[derive(Debug)]
pub struct Config {
    files: Option<Vec<String>>,
    print_mode: PrintMode,
}

pub fn get_config() -> MyResult<Config> {
    Args::parse().to_config()
}


fn open(input: Option<&str>) -> MyResult<Box<dyn BufRead>> {
    match input {
        None => Ok(Box::new(BufReader::new(io::stdin()))),
        Some(filename) => Ok(Box::new(BufReader::new(File::open(filename)?)))
    }   
}

fn print_head(filename: &str, mut buf_reader: Box<dyn BufRead>, print_mode: &PrintMode, not_head: bool, multi_file_flg: bool) {
    // 先頭のイテレータではない場合、空行を出力する
    if not_head {
        println!("");
    } 

    // 複数のfileが指定されていた場合は各ファイルの出力にヘッダーをつける
    if multi_file_flg {
        println!("==> {} <==", filename);
    }

    match print_mode {
        PrintMode::LineMode(n) => {
            let mut line = String::new();
            for _ in 0..*n {
                let bytes = buf_reader.read_line(&mut line)
                    .expect("error while reading the file");
                if bytes == 0 {
                    break;
                }
                print!("{}", line);
                line.clear();
            }
        },
        PrintMode::ByteMode(n) => {
            let bytes  = buf_reader.bytes().take(*n).collect::<Result<Vec<_>, _>>().expect("error while reading bytes");
            print!("{}", String::from_utf8_lossy(&bytes))
        }
    }

}


pub fn run(config: Config) -> MyResult<()> {
    // 少なくとも1つの処理でエラーが発生したか否か
    let mut err_flg = false;

    match config.files {
        None => {
            match open(None) {
                Err(err) => {
                    eprintln!("stdin: {}", err);
                    err_flg = true;
                },
                Ok(buf_reader) => {
                    print_head("not used", buf_reader, &config.print_mode, false, false);
                }
            }

        }
        Some(files) => {
            // 入力ファイルの数が複数あるか
            let multi_file_flg = files.len() > 1;

            files.iter().fold(false, |not_head, filename| {
                // not_head: 先頭のイテレートではない、またその時のみ true
                match open(Some(filename)) {
                    Err(err) => {
                        eprintln!("{}: {}", filename, err);
                        err_flg = true;
                        true
                    },
                    Ok(buf_reader) => {
                        print_head(filename, buf_reader, &config.print_mode, not_head, multi_file_flg);
                        true
                    }
                }
            });
        }
    }

    if err_flg {
        Err(From::from("少なくとも1つのファイルに対してエラーが発生しました"))
    } else {
        Ok(())
    }
}