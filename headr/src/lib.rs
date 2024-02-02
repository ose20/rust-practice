use clap::{App, Arg};
use std::{error::Error, fs::File, io::{self, BufRead, BufReader, Read}};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
enum PrintMode {
    LineMode(usize),
    ByteMode(usize),
}

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    print_mode: PrintMode,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("headr")
        .version("0.1.0")
        .author("ose20 <ose20dive@gmail.com>")
        .about("Rust head")
        .arg(
            Arg::with_name("files")
            .value_name("FILE")
            .help("Input file(s)")
            .multiple(true)
            .default_value("-"),
        )
        .arg(
            Arg::with_name("line_count")
            .short("n")
            .long("lines")
            .value_name("LINES")
            .help("Number of lines")
            .takes_value(true)
            .default_value("10")
            // ここに conflict_withを書くと -c オプションが使えなくなr
        )
        .arg(
            Arg::with_name("byte_count")
            .short("c")
            .long("bytes")
            .value_name("BYTES")
            .help("Number of bytes")
            .takes_value(true)
            .conflicts_with("line_count")
        )
        .get_matches();

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        print_mode: {
            let lines = matches.value_of("line_count")
                .map(parse_positive_int)
                .transpose()
                .map_err(|e| format!("illegal line count -- {}", e))?
                .unwrap();
            let bytes = matches.value_of("byte_count")
                .map(parse_positive_int)
                .transpose()
                .map_err(|e| format!("illegal byte count -- {}", e))?;


            match (lines, bytes) {
                (_, Some(bytes)) => PrintMode::ByteMode(bytes),
                (lines, _) => PrintMode::LineMode(lines),
            }
        }
        
        
    })
}

pub fn run(config: Config) -> MyResult<()> {
    // 少なくとも1つのファイル処理でエラーが発生したか否か
    let mut err_flg = false;
    let multi_file = config.files.len() > 1;

    config.files.iter().fold(false, |not_head, filename| {
        // not_head: 先頭のイテレートではない、またその時のみ true
        match open(filename) {
            Err(err) => {
                eprintln!("{}: {}", filename, err);
                err_flg = true;
                true
            },
            Ok(buf_reader) => {
                print_head(filename, buf_reader, &config.print_mode, not_head, multi_file);
                true
            }
        }
    });

    if err_flg {
        Err(From::from("少なくとも1つのファイルに対してエラーが発生しました"))
    } else {
        Ok(())
    }
}

fn print_head(filename: &str, mut buf_reader: Box<dyn BufRead>, print_mode: &PrintMode, not_head: bool, multi_file: bool) {
    // 先頭のイテレータではない場合、空行を出力する
    if not_head {
        println!("");
    } 

    // 複数のfileが指定されていた場合は各ファイルの出力にヘッダーをつける
    if multi_file {
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
            // let mut handle = buf_reader.take(*n as u64);
            // let mut buffer = vec![0; *n];
            // let bytes_read = handle.read(&mut buffer)
            //     .expect("error while reading bytes from files");
            // print!("{}", String::from_utf8_lossy(&buffer[..bytes_read]));
            let bytes  = buf_reader.bytes().take(*n).collect::<Result<Vec<_>, _>>().expect("error while reading bytes");
            print!("{}", String::from_utf8_lossy(&bytes))
        }
    }

}

fn parse_positive_int(val: &str) -> MyResult<usize> {
    match val.parse() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(From::from(val)),
    }
}

#[test]
fn test_parse_positive_int() {
    let res = parse_positive_int("3");
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 3);

    // 数字でない文字列の場合はエラー
    let res = parse_positive_int("foo");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "foo".to_string());

    // 0の場合もエラー
    let res = parse_positive_int("0");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "0".to_string());
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?)))
    }   
}