use clap::{App, Arg};
use std::error::Error;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: usize,
    bytes: Option<usize>,
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
            .help("Number of lines [default: 10]")
            .takes_value(true)
            .default_value("10")
            .conflicts_with("byte_count"),
        )
        .arg(
            Arg::with_name("byte_count")
            .short("c")
            .long("bytes")
            .value_name("BYTES")
            .help("Number of bytes")
            .takes_value(true),
        )
        .get_matches();

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        lines: matches.value_of("line_count")
            .map(parse_positive_int)
            .transpose()
            .map_err(|e| format!("illegal line count -- {}", e))?
            .unwrap(),
        bytes: matches.value_of("byte_count")
            .map(parse_positive_int)
            .transpose()
            .map_err(|e| format!("illegal byte count -- {}", e))?
    })
}

pub fn run(config: Config) -> MyResult<()> {
    println!("{:#?}", config);
    Ok(())
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
    let res = parse_positive_int("foo");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "0".to_string());
}

