use crate::Extract::*;
use std::{error::Error, fs::File, io::{self, BufRead, BufReader}, num::NonZeroUsize, ops::Range};

use clap::Parser;
use csv::{ReaderBuilder, StringRecord};
use regex::Regex;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Input file(s) [stdin if not specified]
    #[arg(value_name = "FILE")]
    files: Option<Vec<String>>,

    // -fオプション指定された時しか意味がないので、型の設計からそれがちゃんとわかるようにしたい。サブコマンド化かな...
    /// Field delimiter
    #[arg(short, long, value_name = "DELILMITER", default_value = "\t")]
    delimiter: String,

    /// Selected fields
    #[arg(
        short,
        long,
        value_name = "FIELDS",
        conflicts_with_all(["bytes", "chars"])
    )]
    fields: Option<String>,

    /// Selected bytes
    #[arg(
        short,
        long,
        value_name = "BYTES",
        conflicts_with_all(["fields", "chars"])
    )]
    bytes: Option<String>,

    /// Selected chars
    #[arg(
        short,
        long,
        value_name = "CHARS",
        conflicts_with_all(["fields", "bytes"])
    )]
    chars: Option<String>,
}

impl Args {
    fn to_config(self) -> MyResult<Config> {
        let delim_bytes = self.delimiter.as_bytes();
        if delim_bytes.len() != 1 {
            return Err(From::from(format!("--delim \"{}\" must be a single byte", self.delimiter)))
        }
        let delimiter: u8 = *delim_bytes.first().unwrap();

        let extract =
            if let Some(fields) = self.fields.map(parse_pos).transpose()? {
                Fields(fields)
            } else if let Some(bytes) = self.bytes.map(parse_pos).transpose()? {
                Bytes(bytes)
            } else if let Some(chars) = self.chars.map(parse_pos).transpose()? {
                Chars(chars)
            } else {
                return Err(From::from("Must have --fields, --bytes, or --chars"))
            };


        Ok(Config {
            files: self.files,
            delimiter,
            extract,
        })
    }
}

type PositionList = Vec<Range<usize>>;

#[derive(Debug)]
enum Extract {
    Fields(PositionList),
    Bytes(PositionList),
    Chars(PositionList),
}

#[derive(Debug)]
pub struct Config {
    files: Option<Vec<String>>,
    delimiter: u8,
    extract: Extract,
}

enum Input {
    Stdin,
    File(String),
}

fn parse_index(input: &str) -> Result<usize, String> {
    let value_error = || format!("illegal list value: \"{}\"", input);
    input
        .starts_with('+')
        .then(|| Err(value_error()))
        .unwrap_or_else(|| {
            input
                .parse::<NonZeroUsize>()
                .map(|n| usize::from(n) - 1)
                .map_err(|_| value_error())
        })
}

fn parse_pos(range: String) -> MyResult<PositionList> {
    let range_re = Regex::new(r"^(\d+)-(\d+)$").unwrap();
    range
        .split(',')
        .into_iter()
        .map(|val| {
            parse_index(val).map(|n| n..n+1).or_else(|e| {
                range_re.captures(val).ok_or(e).and_then(|captures| {
                    let n1 = parse_index(&captures[1])?;
                    let n2 = parse_index(&captures[2])?;
                    if n1 >= n2 {
                        return Err(format!(
                            "First number in range ({}) \
                            must be lower than second number ({})",
                            n1 + 1,
                            n2 + 1
                        ));
                    }
                    Ok(n1..n2+1)
                })
            })
        })
        .collect::<Result<_, _>>()
        .map_err(From::from)

}

fn open(input: Input) -> MyResult<Box<dyn BufRead>> {
    match input {
        Input::Stdin => Ok(Box::new(BufReader::new(io::stdin()))),
        Input::File(file) => Ok(Box::new(BufReader::new(File::open(file)?)))
    }
}

fn extract_fields(record: &StringRecord, field_pos: &[Range<usize>]) -> Vec<String> {
    // 指定された range に含まれる field のリストを返す。見つからなかった場合は None を返す
    let subfield = |record: &StringRecord, range: Range<usize>| -> Option<Vec<String>> {
        let found: Vec<String> = record.iter()
            .enumerate()
            .filter_map(|(i, field)| if range.contains(&i) { Some(field.to_string()) } else { None })
            .collect();

        if found.is_empty() { None } else { Some(found) }
    };

    field_pos.iter()
        .cloned()
        .filter_map(|range| subfield(&record, range))
        .flatten()
        .collect()
}

fn extract_chars(line: &str, char_pos: &[Range<usize>]) -> String {
    let substring = |s: &str, start: usize, end: usize| -> String {
        s.chars()
            .enumerate()
            .filter_map(|(i, c)| if start <= i && i < end { Some(c) } else { None })
            .collect()
    };

    char_pos.iter()
        .map(|range| substring(line, range.start, range.end))
        .collect::<Vec<_>>()
        .join("")
}

fn extract_bytes(line: &str, byte_pos: &[Range<usize>]) -> String {
    let subbytes = |s: &str, range: Range<usize>| -> String {
        let bytes = s.as_bytes();
        String::from_utf8_lossy(bytes.get(range).unwrap_or(b"")).to_string()
    };

    byte_pos.into_iter()
        .cloned()
        .map(|range| subbytes(line, range))
        .collect::<Vec<_>>()
        .join("")
}

pub fn get_config() -> MyResult<Config> {
    Args::parse().to_config()
}

fn print(config: &Config, buf_reader: Box<dyn BufRead>) -> MyResult<()> {
    match &config.extract {
        Fields(ranges) => {
            let mut reader = ReaderBuilder::new()
                .delimiter(config.delimiter)
                .from_reader(buf_reader);

            let header = reader.headers()?;
            let delim = (config.delimiter as char).to_string();
            println!("{}", extract_fields(&header, ranges).join(&delim));
            for record in reader.records() {
                let record = record?;
                println!(
                    "{}", extract_fields(&record, ranges).join(&delim)
                )
            }
            Ok(())
        },
        Bytes(ranges) => {
            for line in buf_reader.lines() {
                let line = line?;
                println!("{}", extract_bytes(line.as_str(), ranges))
            }

            Ok(())
        },
        Chars(ranges) => {
            for line in buf_reader.lines() {
                let line = line?;
                println!("{}", extract_chars(line.as_str(), ranges))
            }

            Ok(())
        }
    }
}

pub fn run(config: Config) -> MyResult<()> {
    match &config.files {
        None => {
            match open(Input::Stdin) {
                Err(err) => eprintln!("stdin: {}", err),
                Ok(buf_reader) => {
                    print(&config, buf_reader)?
                }
            }
        },
        Some(files) => {
            for filename in files {
                match open(Input::File(filename.clone())) {
                    Err(err) => eprintln!("{}: {}", filename, err),
                    Ok(buf_reader) => {
                        print(&config, buf_reader)?
                    }
                }
            }
        }
    }

    Ok(())
}



// ------------------------------------------------------------
#[cfg(test)]
mod unit_tests {
    use csv::StringRecord;

    use crate::extract_fields;

    use super::{extract_chars, extract_bytes, parse_pos};

    #[test]
    fn test_parse_pos() {
        // The empty string is an error
        assert!(parse_pos("".to_string()).is_err());

        // Zero is an error
        let res = parse_pos("0".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "0""#
        );

        let res = parse_pos("0-1".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "0""#
        );

        // A leading "+" is an error
        let res = parse_pos("+1".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "+1""#,
        );

        let res = parse_pos("+1-2".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "+1-2""#,
        );

        let res = parse_pos("1-+2".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "1-+2""#,
        );

        // Any non-number is an error
        let res = parse_pos("a".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "a""#
        );

        let res = parse_pos("1,a".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "a""#
        );

        let res = parse_pos("1-a".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "1-a""#,
        );

        let res = parse_pos("a-1".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            r#"illegal list value: "a-1""#,
        );

        // Wonky ranges
        let res = parse_pos("-".to_string());
        assert!(res.is_err());

        let res = parse_pos(",".to_string());
        assert!(res.is_err());

        let res = parse_pos("1,".to_string());
        assert!(res.is_err());

        let res = parse_pos("1-".to_string());
        assert!(res.is_err());

        let res = parse_pos("1-1-1".to_string());
        assert!(res.is_err());

        let res = parse_pos("1-1-a".to_string());
        assert!(res.is_err());

        // First number must be less than second
        let res = parse_pos("1-1".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (1) must be lower than second number (1)"
        );

        let res = parse_pos("2-1".to_string());
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (2) must be lower than second number (1)"
        );

        // All the following are acceptable
        let res = parse_pos("1".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("01".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("1,3".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("001,0003".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("1-3".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("0001-03".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("1,7,3-5".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 6..7, 2..5]);

        let res = parse_pos("15,19-20".to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![14..15, 18..20]);
    }
    

    #[test]
    fn test_extract_chars() {
        assert_eq!(extract_chars("", &[0..1]), "".to_string());
        assert_eq!(extract_chars("Émile", &[0..1]), "É".to_string());
        assert_eq!(extract_chars("Émile", &[0..1, 2..3]), "Éi".to_string());
        assert_eq!(extract_chars("Émile", &[0..3]), "Émi".to_string());
        assert_eq!(extract_chars("Émile", &[2..3, 1..2]), "im".to_string());
        assert_eq!(extract_chars("Émile", &[0..1, 1..2, 6..7]), "Ém".to_string());
    }

    #[test]
    fn test_extract_bytes() {
        assert_eq!(extract_bytes("ábc", &[0..1]), "�".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2]), "á".to_string());
        assert_eq!(extract_bytes("ábc", &[0..3]), "áb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..4]), "ábc".to_string());
        assert_eq!(extract_bytes("ábc", &[3..4, 2..3]), "cb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2, 5..6]), "á".to_string());
    }

    #[test]
    fn test_extract_fields() {
        let rec = StringRecord::from(vec!["Captain", "Sham", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2]), &["Sham"]);
        assert_eq!(extract_fields(&rec, &[0..1, 2..3]), &["Captain", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1, 3..4]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2, 0..1]), &["Sham", "Captain"]);
        assert_eq!(extract_fields(&rec, &[100..150]), vec!["dummy"; 0]);
        assert_eq!(extract_fields(&rec, &[0..100]), &["Captain", "Sham", "12345"])
    }
}
