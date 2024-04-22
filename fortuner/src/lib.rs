use std::{
    error::Error,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::PathBuf,
};

use clap::Parser;
use rand::{rngs::StdRng, seq::SliceRandom, RngCore, SeedableRng};
use regex::{Regex, RegexBuilder};
use walkdir::WalkDir;

type MyResult<T> = Result<T, Box<dyn Error>>;

// ------------------------------------------------------------------------------------------------
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Args {
    // todo: これは位置引数なので必須にしたい
    #[arg(required = true, value_name = "FILE")]
    sources: Vec<String>,

    /// Pattern
    #[arg(value_name = "PATTERN", short = 'm', long)]
    pattern: Option<String>,

    /// Random seed
    // あとでパースするので一旦文字列で受ける
    #[arg(value_name = "SEED", short, long)]
    seed: Option<String>,

    /// Case-insensitive pattern matching
    #[arg(short, long)]
    insensitive: bool,
}

// ------------------------------------------------------------------------------------------------
#[derive(Debug)]
struct Fortune {
    source: String,
    text: String,
}

// ------------------------------------------------------------------------------------------------
impl Args {
    fn to_config(self) -> MyResult<Config> {
        let pattern = self
            .pattern
            .map(|ptn| {
                RegexBuilder::new(&ptn)
                    .case_insensitive(self.insensitive)
                    .build()
                    .map_err(|_| format!("Invalid pattern \"{}\"", &ptn))
            })
            .transpose()?;

        let seed = self.seed.map(|s| parse_u64(&s)).transpose()?;

        Ok(Config {
            pattern,
            sources: self.sources,
            seed,
        })
    }
}

// ------------------------------------------------------------------------------------------------
fn parse_u64(val: &str) -> MyResult<u64> {
    val.parse()
        .map_err(|_| format!("\"{}\" not a valid integer", val).into())
}

// ------------------------------------------------------------------------------------------------
fn find_files(paths: &[String]) -> MyResult<Vec<PathBuf>> {
    let mut files = vec![];

    for path in paths {
        match fs::metadata(path) {
            Err(e) => return Err(From::from(format!("{}: {}", path, e))),
            Ok(_) => files.extend(
                WalkDir::new(path)
                    .into_iter()
                    .filter_map(Result::ok)
                    .filter(|e| e.file_type().is_file())
                    .map(|e| e.path().into()),
            ),
        }
    }

    files.sort();
    files.dedup();
    Ok(files)
}

// ------------------------------------------------------------------------------------------------
fn read_fortunes(paths: &[PathBuf]) -> MyResult<Vec<Fortune>> {
    let mut fortunes = vec![];
    let mut buffer = vec![];

    for path in paths {
        let basename = path.file_name().unwrap().to_string_lossy().into_owned();
        let file = File::open(path).map_err(|e| format!("{}: {e}", path.to_string_lossy()))?;

        for line in BufReader::new(file).lines().map_while(Result::ok) {
            if line == "%" {
                if !buffer.is_empty() {
                    fortunes.push(Fortune {
                        source: basename.clone(),
                        text: buffer.join("\n"),
                    });
                    buffer.clear();
                }
            } else {
                buffer.push(line.to_string());
            }
        }
    }

    Ok(fortunes)
}

// ------------------------------------------------------------------------------------------------
fn pick_fortune(fortunes: &[Fortune], seed: Option<u64>) -> Option<String> {
    let mut rng: Box<dyn RngCore> = match seed {
        Some(val) => Box::new(StdRng::seed_from_u64(val)),
        _ => Box::new(rand::thread_rng()),
    };

    fortunes.choose(&mut rng).map(|f| f.text.to_string())
}

// ------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct Config {
    sources: Vec<String>,
    pattern: Option<Regex>,
    seed: Option<u64>,
}

// ------------------------------------------------------------------------------------------------
pub fn get_config() -> MyResult<Config> {
    Args::parse().to_config()
}

// ------------------------------------------------------------------------------------------------
pub fn run(config: Config) -> MyResult<()> {
    let files = find_files(&config.sources)?;
    let fortunes = read_fortunes(&files)?;
    match config.pattern {
        Some(pattern) => {
            let mut prev_source = None;
            for fortune in fortunes
                .iter()
                .filter(|fortune| pattern.is_match(&fortune.text))
            {
                if prev_source.as_ref().map_or(true, |s| s != &fortune.source) {
                    eprintln!("({})\n%", fortune.source);
                    prev_source = Some(fortune.source.clone())
                }
                println!("{}\n%", fortune.text);
            }
        }
        _ => {
            println!(
                "{}",
                pick_fortune(&fortunes, config.seed)
                    .or_else(|| Some("No fortunes found".to_string()))
                    .unwrap()
            )
        }
    }

    Ok(())
}

// ------------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use assert_cmd::assert;

    use crate::find_files;

    #[test]
    fn test_find_files() {
        // 存在するファイルの検索
        let res = find_files(&["./tests/inputs/jokes".to_string()]);
        assert!(res.is_ok());

        let files = res.unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(
            files.get(0).unwrap().to_string_lossy(),
            "./tests/inputs/jokes"
        );

        // 存在しないファイルの検索
        let res = find_files(&["/path/does/not/exist".to_string()]);
        assert!(res.is_err());

        // 拡張子が .dat 以外の入力ファイルをすべて検索する
        let res = find_files(&["./tests/inputs".to_string()]);
        assert!(res.is_ok());

        // ファイル数とファイルの順番の確認
        let files = res.unwrap();
        assert_eq!(files.len(), 5);
        let first = files.get(0).unwrap().display().to_string();
        assert!(first.contains("ascii-art"));
        let last = files.last().unwrap().display().to_string();
        assert!(last.contains("quotes"));

        // ソースが複数ある場合
        // 重複なしでソートされている
        let res = find_files(&[
            "./tests/inputs/jokes".to_string(),
            "./tests/inputs/ascii-art".to_string(),
            "./tests/inputs/jokes".to_string(),
        ]);
        assert!(res.is_ok());
        let files = res.unwrap();
        assert_eq!(files.len(), 2);
        if let Some(filename) = files.first().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "ascii-art".to_string())
        }
        if let Some(filename) = files.last().unwrap().file_name() {
            assert_eq!(filename.to_string_lossy(), "jokes".to_string())
        }
    }
}
