mod owner;

use std::{error::Error, fs, os::unix::fs::MetadataExt, path::PathBuf};

use chrono::{DateTime, Local};
use clap::Parser;
use owner::Owner;
use tabular::{Row, Table};
use users::{get_group_by_gid, get_user_by_uid};

type MyResult<T> = Result<T, Box<dyn Error>>;

// ------------------------------------------------------------------------------------------------
#[derive(Debug, Parser)]
#[command(author, version, about)]
struct Args {
    /// arg paths
    #[arg(value_name = "PATH", default_value = ".")]
    paths: Vec<String>,

    /// Long listing
    #[arg(short, long)]
    long: bool,

    /// Show all files
    #[arg(short = 'a', long = "all")]
    show_hidden: bool,
}

// ------------------------------------------------------------------------------------------------
pub fn run() -> MyResult<()> {
    let config = Args::parse();
    let paths = find_files(&config.paths, config.show_hidden)?;

    if config.long {
        println!("{}", format_output(&paths)?)
    } else {
        for path in paths {
            println!("{}", path.display());
        }
    }

    Ok(())
}

// ------------------------------------------------------------------------------------------------
/// paths の各エントリに対し、file ならそのまま、dir ならその要素のリストを取得して、それらを flat　にして返す関数
/// 存在しなかったり取得できない場合はその都度エラー出力がなされ、処理は止まらない
fn find_files(paths: &[String], show_hidden: bool) -> MyResult<Vec<PathBuf>> {
    let mut pathbufs = Vec::new();

    for path in paths.iter() {
        match fs::metadata(path) {
            Ok(metadate) => {
                if metadate.is_file() {
                    // file　の場合
                    pathbufs.push(PathBuf::from(path));
                } else if metadate.is_dir() {
                    // dir の場合
                    add_entries(&mut pathbufs, path);
                } else {
                    // おそらく symlink?
                    eprintln!("skip: path is not file or dir. Is this symlink? {}", path);
                }
            }
            Err(e) => {
                eprintln!("err: metadataの取得\n{:#?}", e);
            }
        }
    }

    // show_hiddenがない場合は dotfile を捨てる
    if !show_hidden {
        pathbufs.retain(|pathbuf| {
            !pathbuf
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("."))
                .unwrap_or(false)
        });
    }

    Ok(pathbufs)
}

// ------------------------------------------------------------------------------------------------
fn add_entries(pathbufs: &mut Vec<PathBuf>, path: &String) {
    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries {
                match entry {
                    Ok(entry) => pathbufs.push(PathBuf::from(entry.path())),
                    Err(e) => {
                        eprintln!("err & skip: エントリの取得\n{:#?}", e)
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("err & skip: ディレクトリの読み込み\n{:#?}", e);
        }
    }
}

// ------------------------------------------------------------------------------------------------
fn format_output(paths: &[PathBuf]) -> MyResult<String> {
    //               1   2     3     4     5     6     7     8
    let fmt = "{:<}{:<}  {:>}  {:<}  {:<}  {:>}  {:<}  {:<}";
    let mut table = Table::new(fmt);

    for path in paths {
        let metadata = fs::metadata(path)?;
        let uid = metadata.uid();
        let user = get_user_by_uid(uid)
            .map(|u| u.name().to_string_lossy().into_owned())
            .unwrap_or_else(|| uid.to_string());

        let gid = metadata.gid();
        let group = get_group_by_gid(gid)
            .map(|g| g.name().to_string_lossy().into_owned())
            .unwrap_or_else(|| gid.to_string());

        let modified: DateTime<Local> = DateTime::from(metadata.modified()?);

        table.add_row(
            Row::new()
                .with_cell(if path.is_dir() { "d" } else { "-" })
                .with_cell(format_mode(metadata.mode()))
                .with_cell(metadata.nlink())
                .with_cell(user)
                .with_cell(group)
                .with_cell(metadata.len())
                .with_cell(modified.format("%b %d %y %H:%M"))
                .with_cell(path.display()),
        );
    }

    Ok(format!("{}", table))
}

// ------------------------------------------------------------------------------------------------
/// 0o761のような8進数でファイルモードを指定すると、
/// 「rwxr-x--x」のような文字列を返す
fn format_mode(mode: u32) -> String {
    // 0 1 2 3 4 5 6 7 8
    // r w x r w x r w x
    let mut res = vec!['_'; 9];

    // これ、res を &mut としてもらったら set_mode を　 mut にしなくていい？ ← いい
    let set_mode = |owner: Owner, offset: usize, res: &mut Vec<char>| {
        for (i, (mask, mark)) in owner.masks().iter().zip(['r', 'w', 'x']).enumerate() {
            res[offset + i] = if mask & mode != 0 { mark } else { '-' }
        }
    };

    set_mode(Owner::User, 0, &mut res);
    set_mode(Owner::Group, 3, &mut res);
    set_mode(Owner::Other, 6, &mut res);

    res.iter().collect()
}

// ------------------------------------------------------------------------------------------------
#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use crate::{find_files, format_mode, format_output};

    #[test]
    fn test_find_files() {
        // ディレクトリにある隠しエントリ以外のエントリを検索する
        let res = find_files(&["tests/inputs".to_string()], false);
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|e| e.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt",
            ]
        );

        // 隠しファイルも含めて検索
        let res = find_files(&["tests/inputs/.hidden".to_string()], true);
        assert!(res.is_ok());
        let filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|e| e.display().to_string())
            .collect();
        assert_eq!(filenames, ["tests/inputs/.hidden"]);

        // 複数のパス
        let res = find_files(
            &[
                "tests/inputs/bustle.txt".to_string(),
                "tests/inputs/dir".to_string(),
            ],
            false,
        );
        assert!(res.is_ok());
        let mut filenames: Vec<_> = res
            .unwrap()
            .iter()
            .map(|e| e.display().to_string())
            .collect();
        filenames.sort();
        assert_eq!(
            filenames,
            ["tests/inputs/bustle.txt", "tests/inputs/dir/spiders.txt"]
        );
    }

    #[test]
    fn test_find_files_hidden() {
        // ディレクトリ内の全てのエントリを検索する
        let res = find_files(&["tests/inputs".to_string()], true);
        assert!(res.is_ok());
        let mut filenames = res
            .unwrap()
            .iter()
            .map(|e| e.display().to_string())
            .collect::<Vec<_>>();
        filenames.sort();
        assert_eq!(
            filenames,
            [
                "tests/inputs/.hidden",
                "tests/inputs/bustle.txt",
                "tests/inputs/dir",
                "tests/inputs/empty.txt",
                "tests/inputs/fox.txt"
            ]
        )
    }

    #[test]
    fn test_format_mode() {
        assert_eq!(format_mode(0o755), "rwxr-xr-x");
        assert_eq!(format_mode(0o421), "r---w---x");
    }

    #[test]
    fn test_format_output_one() {
        let bustle_path = "tests/inputs/bustle.txt";
        let bustle = PathBuf::from(bustle_path);

        let res = format_output(&[bustle]);
        assert!(res.is_ok());

        let out = res.unwrap();
        let lines: Vec<&str> = out.split("\n").filter(|s| !s.is_empty()).collect();
        assert_eq!(lines.len(), 1);

        let line1 = lines.first().unwrap();
        long_match(&line1, &bustle_path, "-rw-r--r--", Some("193"));
    }

    #[test]
    fn test_format_output_two() {
        let res = format_output(&[
            PathBuf::from("tests/inputs/dir"),
            PathBuf::from("tests/inputs/empty.txt"),
        ]);
        assert!(res.is_ok());

        let out = res.unwrap();
        let mut lines: Vec<&str> = out.split("\n").filter(|s| !s.is_empty()).collect();
        lines.sort();
        assert_eq!(lines.len(), 2);
        let empty_line = lines.remove(0);
        long_match(
            &empty_line,
            "tests/inputs/empty.txt",
            "-rw-r--r--",
            Some("0"),
        );

        let dir_line = lines.remove(0);
        long_match(&dir_line, "tests/inputs/dir", "drwxr-xr-x", None);
    }

    fn long_match(
        line: &str,
        expected_name: &str,
        expected_perms: &str,
        expected_size: Option<&str>,
    ) {
        let parts: Vec<_> = line.split_whitespace().collect();
        assert!(parts.len() > 0 && parts.len() <= 10);

        let perms = parts.get(0).unwrap();
        assert_eq!(perms, &expected_perms);

        if let Some(size) = expected_size {
            let file_size = parts.get(4).unwrap();
            assert_eq!(file_size, &size);
        }

        let display_name = parts.last().unwrap();
        assert_eq!(display_name, &expected_name);
    }
}
