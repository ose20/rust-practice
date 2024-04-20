use clap::Parser;
use once_cell::sync::OnceCell;
use regex::Regex;
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Read, Seek},
};
use TakeValue::*;

// ------------------------------------------------------------------------------------------------
static NUM_RE: OnceCell<Regex> = OnceCell::new();

// ------------------------------------------------------------------------------------------------
type MyResult<T> = Result<T, Box<dyn Error>>;

// ------------------------------------------------------------------------------------------------
#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Input file(s)
    #[arg(required = true)]
    files: Vec<String>,

    /// Number of lines
    #[arg(value_name = "LINES", short = 'n', long, default_value = "10")]
    lines: String,

    /// Number of bytes
    #[arg(value_name = "BYTES", short = 'c', long, conflicts_with("lines"))]
    bytes: Option<String>,

    /// Suppress headers
    #[arg(short, long)]
    quiet: bool,
}

// ------------------------------------------------------------------------------------------------
impl Args {
    fn to_config(self) -> MyResult<Config> {
        let files = self.files;
        let quiet = self.quiet;

        let tail_mode = if let Some(num) = self.bytes {
            TailMode::Bytes(parse_num(&num).map_err(|e| format!("illegal byte count -- {}", e))?)
        } else {
            TailMode::Lines(
                parse_num(&self.lines).map_err(|e| format!("illegal line count -- {}", e))?,
            )
        };

        Ok(Config {
            files,
            quiet,
            tail_mode,
        })
    }
}

// ------------------------------------------------------------------------------------------------
#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    tail_mode: TailMode,
    quiet: bool,
}

// ------------------------------------------------------------------------------------------------
#[derive(Debug, Clone, Copy)]
enum TailMode {
    Lines(TakeValue),
    Bytes(TakeValue),
}

// ------------------------------------------------------------------------------------------------
#[derive(Debug, PartialEq, Clone, Copy)]
enum TakeValue {
    PlusZero,
    TakeNum(i64),
}

// ------------------------------------------------------------------------------------------------
fn parse_num(val: &str) -> MyResult<TakeValue> {
    let num_re = NUM_RE.get_or_init(|| Regex::new(r"^([+-])?(\d+)$").unwrap());

    match num_re.captures(val) {
        Some(caps) => {
            let sign = caps.get(1).map_or("-", |m| m.as_str());
            let num = format!("{}{}", sign, caps.get(2).unwrap().as_str());
            if let Ok(val) = num.parse() {
                if sign == "+" && val == 0 {
                    Ok(PlusZero)
                } else {
                    Ok(TakeNum(val))
                }
            } else {
                Err(From::from(val))
            }
        }
        _ => Err(From::from(val)),
    }
}

// ------------------------------------------------------------------------------------------------
pub fn get_config() -> MyResult<Config> {
    Args::parse().to_config()
}

// ------------------------------------------------------------------------------------------------
// print仕様
// 複数ファイルがある場合、==> filename <== のヘッダーが、存在するファイルのみにつく
// また、成功したファイルの2つ目以降はヘッダーの前に一行空行を入れる
// quietモードの場合、ヘッダーだけでなく空行も出力しない
pub fn run(config: Config) -> MyResult<()> {
    for (idx, filename) in config.files.iter().enumerate() {
        match File::open(filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(file) => {
                let is_multi = config.files.len() > 1;
                if is_multi && !config.quiet {
                    println!("{}==> {} <==", if idx > 0 { "\n" } else { "" }, filename);
                }
                let (total_lines, total_bytes) = count_lines_bytes(filename)?;
                let file = BufReader::new(file);
                match config.tail_mode {
                    TailMode::Lines(line_num) => print_lines(file, &line_num, total_lines)?,
                    TailMode::Bytes(byte_num) => print_byte(file, &byte_num, total_bytes)?,
                }
            }
        }
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------
fn count_lines_bytes(filename: &str) -> MyResult<(usize, usize)> {
    let file = File::open(filename)?;
    let mut reader = BufReader::new(file);

    let mut lines = 0;
    let mut bytes = 0;
    let mut buf = Vec::new();

    loop {
        let bytes_read = reader.read_until(b'\n', &mut buf)?;
        if bytes_read == 0 {
            break;
        }
        lines += 1;
        bytes += bytes_read;
        buf.clear();
    }

    Ok((lines, bytes))
}

fn print_lines(mut file: impl BufRead, num_lines: &TakeValue, total_lines: usize) -> MyResult<()> {
    if let Some(start) = get_start_index(num_lines, total_lines) {
        let mut line_num = 1;
        let mut buf = Vec::new();
        loop {
            let bytes_read = file.read_until(b'\n', &mut buf)?;
            if bytes_read == 0 {
                break;
            }
            if line_num >= start {
                print!("{}", String::from_utf8_lossy(&buf));
            }
            line_num += 1;
            buf.clear();
        }
    }

    Ok(())
}

// ------------------------------------------------------------------------------------------------
// print_lineと同様に T を書かずに file: impl Read + Seek　としても良い
// Seek は多くのプログラミング言語で「カーソル」や「読み込みヘッド」と呼ばれるものをストリームの特定の位置に移動させることを意味する
fn print_byte<T>(mut file: T, num_bytes: &TakeValue, total_bytes: usize) -> MyResult<()>
where
    T: Read + Seek,
{
    if let Some(start) = get_start_index(num_bytes, total_bytes) {
        file.seek(std::io::SeekFrom::Start((start - 1) as u64))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        if !buffer.is_empty() {
            print!("{}", String::from_utf8_lossy(&buffer));
        }
    }
    Ok(())
}

// ------------------------------------------------------------------------------------------------
// ユーザが指定した TakeValue と、対象ファイルの大きさを受け取り、開始位置を返す
// 開始位置がファイルの大きさを超えると None が返る
// lineでもbyteでもロジックが同じなので捨象して良い
// 開始位置は 1-origin であることに注意（0も1も同じ意味だけど、0は1に正規化して返す）
// Some(num)は、バイトならnumバイト目以降、行ならnum行以降を表現する
fn get_start_index(take_val: &TakeValue, total: usize) -> Option<usize> {
    match take_val {
        PlusZero => Some(1),
        TakeNum(num) if *num == 0 => None,
        TakeNum(num) if *num > 0 => {
            if *num <= (total as i64) {
                Some(*num as usize)
            } else {
                None
            }
        }
        TakeNum(num) if *num < 0 => {
            // num < 0 なので、 idx <= total が成り立つ
            let idx = (total as i64) + 1 + num;
            if idx <= 0 {
                Some(1)
            } else {
                Some(idx as usize)
            }
        }
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::{count_lines_bytes, get_start_index, parse_num, TakeValue::*};

    #[test]
    fn test_parse_num() {
        // +のついていない整数は負の数として扱う
        let res = parse_num("3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));

        // 先頭に「+」がついている場合は正の数として解釈される
        let res = parse_num("+3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(3));

        let res = parse_num("-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));

        // 0 は 0 のまま
        let res = parse_num("0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(0));

        // +0 は特別扱い
        let res = parse_num("+0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), PlusZero);

        // 境界値テスト
        let res = parse_num(&i64::MAX.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));

        let res = parse_num(&(i64::MIN + 1).to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));

        let res = parse_num(&format!("+{}", i64::MAX));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MAX));

        let res = parse_num(&i64::MIN.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN));

        let res = parse_num("3.14");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "3.14");

        let res = parse_num("nyaa");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "nyaa");
    }

    #[test]
    fn test_count_lines_bytes() {
        let res = count_lines_bytes("tests/inputs/one.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (1, 24));

        let res = count_lines_bytes("tests/inputs/ten.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (10, 49));
    }

    #[test]
    fn test_get_start_index() {
        // 本だとNone返せって言ってるけどこっちの方が一貫性があると思うので変えるぜ
        assert_eq!(get_start_index(&PlusZero, 0), Some(1));

        // +0 from a nonempty file returns an index that
        // is one less than the number of lines/bytes
        assert_eq!(get_start_index(&PlusZero, 1), Some(1));

        // Taking 0 lines/bytes returns None
        assert_eq!(get_start_index(&TakeNum(0), 1), None);

        // Taking any lines/bytes from an empty file returns None
        assert_eq!(get_start_index(&TakeNum(1), 0), None);

        // Taking more lines/bytes than is available returns None
        assert_eq!(get_start_index(&TakeNum(2), 1), None);

        // When starting line/byte is less than total lines/bytes,
        // return one less than starting number
        assert_eq!(get_start_index(&TakeNum(1), 10), Some(1));
        assert_eq!(get_start_index(&TakeNum(2), 10), Some(2));
        assert_eq!(get_start_index(&TakeNum(3), 10), Some(3));

        // When starting line/byte is negative and less than total,
        // return total - start
        assert_eq!(get_start_index(&TakeNum(-1), 10), Some(10));
        assert_eq!(get_start_index(&TakeNum(-2), 10), Some(9));
        assert_eq!(get_start_index(&TakeNum(-3), 10), Some(8));

        // When the starting line/byte is negative and more than the total,
        // return 0 to print the whole file
        assert_eq!(get_start_index(&TakeNum(-20), 10), Some(1));
    }
}
