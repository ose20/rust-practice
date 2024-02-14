use std::{error::Error, fs::{self, File}, io::{self, BufRead, BufReader}, iter::once};

use clap::Parser;
use regex::{Regex, RegexBuilder};
use walkdir::WalkDir;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Args {
    /// Search pattern
    #[arg(value_name = "PATTERN")]
    pattern: String,

    /// Input file(s) [stdin is selected if not specified]
    #[arg(value_name = "FILE")]
    files: Option<Vec<String>>,

    /// Recursive search
    #[arg(short, long)]
    recursive: bool,

    /// Count occurrence
    #[arg(short, long)]
    count: bool,

    /// Invert match
    #[arg(short = 'v', long = "invert-match")]
    invert_match: bool,

    /// Case-insensitive
    #[arg(short, long)]
    insensitive: bool,
}

impl Args {
    fn to_config(self) -> MyResult<Config> {
        let pattern = RegexBuilder::new(&self.pattern)
            .case_insensitive(self.insensitive)
            .build()
            .map_err(|_| format!("Invalid pattern \"{}\"", self.pattern))?;

        Ok(Config {
            pattern,
            files: self.files,
            recursive: self.recursive,
            count: self.count,
            invert_match: self.invert_match,
        })
    }
}

#[derive(Debug)]
pub struct Config {
    pattern: Regex,
    files: Option<Vec<String>>,
    recursive: bool,
    count: bool,
    invert_match: bool,
}

pub fn get_config() -> MyResult<Config> {
    Args::parse().to_config()
}

fn find_files(paths: &[String], recursive: bool) -> Vec<MyResult<String>> {
    let aux = |path: &String| -> Box<dyn Iterator<Item = MyResult<String>>> {
        match fs::metadata(path) {
            Ok(metadata) => {
                if metadata.is_file() {
                    Box::new(once(Ok(path.to_string())))
                } else if metadata.is_dir() {
                    if recursive {
                        let iter = WalkDir::new(path)
                            .into_iter()
                            .filter_map(|dir_entry| {
                                match dir_entry {
                                    Ok(entry) if entry.file_type().is_file() => {
                                        Some(Ok(entry.path().to_string_lossy().into_owned()))
                                    },
                                    Ok(_) => None,
                                    Err(e) => Some(Err(From::from(e))),
                                }
                            });
                        
                        Box::new(iter)
                    } else {
                        Box::new(once(Err(From::from(
                            format!("{} is a directory", path)
                        ))))
                    }
                } else {
                    Box::new(once(Err(From::from(
                        format!("{} this is not file or dir. Maybe link?", path)
                    ))))
                }
            },
            Err(e) => {
                Box::new(once(Err(From::from(
                    format!("{}: {}", path, e)
                ))))
            }
        }
    };

    paths
        .into_iter()
        .flat_map(|path| aux(path))
        .collect()
}

fn find_lines<T: BufRead> (
    mut file: T,
    pattern: &Regex,
    invert_match: bool,
) -> MyResult<Vec<String>> {

    let mut result: Vec<String> = Vec::new();

    loop {
        let mut line_buf = String::new();
        let bytes = file.read_line(&mut line_buf)?;
        if bytes == 0 { break; }

        match (pattern.is_match(&line_buf), invert_match) {
            (true, false) | (false, true) => { result.push(line_buf) }
            _ => {}
        }
    }

    Ok(result)
}

fn print_lines(header: Option<&str>, lines: Vec<String>, count: bool) {
    let header = if let Some(file) = header { format!("{}:", file) } else { "".to_string() };

    if count {
        println!("{}{}", header, lines.len());
    } else {
        for line in lines {
            print!("{}{}", header, line)
        }
    }

}

fn open(input: Option<&str>) -> MyResult<Box<dyn BufRead>> {
    match input {
        None => Ok(Box::new(BufReader::new(io::stdin()))),
        Some(file) => Ok(Box::new(BufReader::new(File::open(file)?)))
    }
}

pub fn run(config: Config) -> MyResult<()> {

    match config.files {
        None => {
            let buf_reader = open(None)?;
            let result_lines = find_lines(buf_reader, &config.pattern, config.invert_match)?;
            print_lines(None, result_lines, config.count);
        },
        Some(paths) => {
            let files = find_files(&paths, config.recursive);
            for entry in &files {
                match entry {
                    Err(e) => eprintln!("{}", e),
                    Ok(filename) => {
                        let buf_reader = open(Some(&filename))?;
                        let result_lines = find_lines(buf_reader, &config.pattern, config.invert_match)?;
                        print_lines(
                            if files.len()>1 { Some(&filename) } else { None },
                            result_lines,
                            config.count
                        )
                    }
                }
            }
        }
    }

    Ok(())
}


#[cfg(test)]
mod tests {
    use std::io::Cursor;


    use super::{find_files, find_lines};
    use rand::{distributions::Alphanumeric, Rng};
    use regex::{Regex, RegexBuilder};

    #[test]
    fn test_find_files() {
        // 存在するファイルを見つけられる
        let files = find_files(&["./tests/inputs/fox.txt".to_string()], false);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].as_ref().unwrap(), "./tests/inputs/fox.txt");

        // recursiveなしの場合、ディレクトリを拒否する
        let files = find_files(&["./tests/inputs".to_string()], false);
        assert_eq!(files.len(), 1);
        if let Err(e) = &files[0] {
            assert_eq!(e.to_string(), "./tests/inputs is a directory");
        }

        // ディレクトリ内の4つのファイルを再帰的に検索できることを確認する
        let res = find_files(&["./tests/inputs".to_string()], true);
        let mut files: Vec<String> = res
            .iter()
            .map(|r| r.as_ref().unwrap().replace("\\", "/"))
            .collect();
        files.sort();
        assert_eq!(files.len(), 4);
        assert_eq!(
            files,
            vec![
                "./tests/inputs/bustle.txt",
                "./tests/inputs/empty.txt",
                "./tests/inputs/fox.txt",
                "./tests/inputs/nobody.txt",
            ]
        );

        // 存在しないファイルに対してエラーを返す
        let bad: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();

        let files = find_files(&[bad], false);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_err());
    }

    #[test]
    fn test_find_lines() {
        let text = b"Lorem\nIpsum\r\nDOLOR";
        
        // "or"
        let re1 = Regex::new("or").unwrap();
        let matches = find_lines(Cursor::new(&text), &re1, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);

        // "or" でマッチを反転
        let matches = find_lines(Cursor::new(&text), &re1, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // 大文字と小文字を区別しない正規表現
        let re2 = RegexBuilder::new("or")
            .case_insensitive(true)
            .build()
            .unwrap();

        // "or"
        let matches = find_lines(Cursor::new(&text), &re2, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);

        // "or" でマッチを反転
        let matches = find_lines(Cursor::new(&text), &re2, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);
    }
}