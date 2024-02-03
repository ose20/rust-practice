use std::{error::Error, fs::File, io::{self, BufRead, BufReader}, ops::{Add, AddAssign}};

use clap::{App, Arg};


type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
enum ByteOrChar {
    Byte,
    Char,
    None,
}

// line, word, byte, char オプションについて
//  1. 何も指定されないがない場合は line, word, byte の3つを表示する
//  2. それ以外は指定されたもののみを表示する
//  3. ただし、 byte と char は共存できない
#[derive(Debug)]
pub struct Config {
    files: Option<Vec<String>>,
    lines: bool,
    words: bool,
    bytes_or_chars: ByteOrChar
}



#[derive(Debug, PartialEq)]
pub struct FileInfo {
    num_lines: usize,
    num_words: usize,
    num_bytes: usize,
    num_chars: usize,
}

impl Add for FileInfo {
    type Output = FileInfo;

    fn add(self, rhs: Self) -> Self::Output {
        FileInfo {
            num_lines: self.num_lines + rhs.num_lines,
            num_words: self.num_words + rhs.num_words,
            num_bytes: self.num_bytes + rhs.num_bytes,
            num_chars: self.num_chars + rhs.num_chars,
        }
    }
}

impl AddAssign<&FileInfo> for FileInfo {
    fn add_assign(&mut self, rhs: &Self) {
        self.num_lines += rhs.num_lines;
        self.num_words += rhs.num_words;
        self.num_bytes += rhs.num_bytes;
        self.num_chars += rhs.num_chars;
    }
}

impl FileInfo {
    fn zero() -> FileInfo {
        FileInfo {
            num_lines: 0,
            num_words: 0,
            num_bytes: 0,
            num_chars: 0,
        }
    }
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("wcr")
        .version("0.1.0")
        .author("ose20 <ose20dive@gmail.com>")
        .about("Rust wc")
        .arg(
            Arg::with_name("files")
            .value_name("FILE")
            .help("Input file(s)")
            .multiple(true)
        )
        .arg(
            Arg::with_name("lines")
            .short("l")
            .long("lines")
            .help("Show line count")
            .takes_value(false)
        )
        .arg(
            Arg::with_name("words")
            .short("w")
            .long("words")
            .help("Show word count")
            .takes_value(false)
        )
        .arg(
            Arg::with_name("bytes")
            .short("c")
            .long("bytes")
            .help("Show byte count")
            .takes_value(false)
        )
        .arg(
            Arg::with_name("chars")
            .short("m")
            .long("chars")
            .help("Show character count")
            .conflicts_with("bytes")
            .takes_value(false)
        )
        .get_matches();

    let files = matches.values_of_lossy("files");
    let config = match (matches.is_present("lines"), matches.is_present("words"), matches.is_present("bytes"), matches.is_present("chars")) {
        (false, false, false, false) => Config {
            files,
            lines: true,
            words: true,
            bytes_or_chars: ByteOrChar::Byte,
        },
        // (bytes, chars) の4パターンで総当たり
        (lines, words, true, false) => Config {
            files,
            lines,
            words,
            bytes_or_chars: ByteOrChar::Byte,

        },
        (lines, words, false, true) => Config {
            files,
            lines,
            words,
            bytes_or_chars: ByteOrChar::Char
        },
        (lines, words, false, false) => Config {
            files,
            lines,
            words,
            bytes_or_chars: ByteOrChar::None,

        },
        (_, _, true, true) => panic!("words option conflicts with bytes"),
    };

    Ok(config)
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    Ok(Box::new(BufReader::new(File::open(filename)?)))
}

pub fn count(mut file: impl BufRead) -> MyResult<FileInfo> {
    let mut num_lines = 0;
    let mut num_words = 0;
    let mut num_bytes = 0;
    let mut num_chars = 0;
    let mut line_buf = String::new();

    loop {
        let bytes = file.read_line(&mut line_buf)?;
        if bytes == 0 { break; }

        num_bytes += bytes;
        num_lines += 1;
        num_words += line_buf.split_whitespace().count();
        num_chars += line_buf.chars().count();
        line_buf.clear();
    }


    Ok(FileInfo {
        num_lines,
        num_words,
        num_bytes,
        num_chars,
    })
}

// configの設定がtrueになっているフィールドだけ {:>8} のフォーマットで左から並べ、ファイル名があれば添えて出力する
fn print_info(config: &Config, file_info: &FileInfo, filename: Option<&str>) {
    let mut format = String::from("");
    if config.lines {
        format += &format!("{:>8}", file_info.num_lines);
    }
    if config.words {
        format += &format!("{:>8}", file_info.num_words);
    }
    match config.bytes_or_chars {
        ByteOrChar::Byte => { format += &format!("{:>8}", file_info.num_bytes); },
        ByteOrChar::Char => { format += &format!("{:>8}", file_info.num_chars); },
        ByteOrChar::None => {},
    }

    match filename {
        Some(filename) => println!("{} {}", format, filename),
        None => println!("{}", format),
    }

}

pub fn run(config: Config) -> MyResult<()> {
    match &config.files {
        None => {
            let buf_reader = BufReader::new(io::stdin());
            let file_info = count(buf_reader)?;
            print_info(&config, &file_info, None);
            
        },
        Some(files) => {
            let mut total_info = FileInfo::zero();
            for filename in files {
                match open(&filename) {
                    Err(err) => eprintln!("{}: {}", filename, err),
                    Ok(buf_reader) => {
                        let file_info = count(buf_reader)?;
                        total_info += &file_info;
                        print_info(&config, &file_info, Some(&filename));

                    }
                }
            }
            // fileが複数指定されていた場合はtotalを表示する
            if files.len() > 1 {
                print_info(&config, &total_info, Some("total"));
            }
        }
    }
    Ok(())
}



#[cfg(test)]
mod tests {
    use super::{count, FileInfo};
    use std::io::Cursor;

    #[test]
    fn test_count() {
        let text = "I don't want the world. I just want your half.\r\n";
        let info = count(Cursor::new(text));
        assert!(info.is_ok());
        let expected = FileInfo {
            num_lines: 1,
            num_words: 10,
            num_chars: 48,
            num_bytes: 48,
        };
        assert_eq!(info.unwrap(), expected);
    }
}

