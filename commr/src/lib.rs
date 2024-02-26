use std::{error::Error, fs::File, io::{self, BufRead, BufReader, Lines}};

use clap::{ArgAction, Parser};

type MyResult<T> = Result<T, Box<dyn Error>>;


#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Args {
    /// Input File 1
    file1: String,

    /// Input File 2
    file2: String,

    /// Suppress printing of column 1
    #[arg(
        short = '1',
        action = ArgAction::SetFalse
    )]
    show_col1: bool,
    /// Suppress printing of column 2

    #[arg(
        short = '2',
        action = ArgAction::SetFalse
    )]
    show_col2: bool,
     
    /// Suppress printing of column 3
    #[arg(
        short = '3',
        action = ArgAction::SetFalse
    )]
    show_col3: bool,

    /// Case-insensitive comparison of lines
    #[arg(short = 'i')]
    insensitive: bool,

    #[arg(
        short = 'd',
        long = "output-delimiter",
        default_value = "\t"
    )]
    delimiter: String
}

pub fn get_args() -> MyResult<Args> {
    let args = Args::parse();

    if args.file1 == "-" && args.file2 == "-" {
        Err(From::from("Both input files can't be STDIN (\"-\")"))
    } else {
        Ok(args)
    }
}

pub fn run(args: Args) -> MyResult<()> {
    let mut iter1 = open(&args.file1)?.lines();
    let mut iter2 = open(&args.file2)?.lines();
    let comm_result = proc_lines(&mut iter1, &mut iter2, &args)?;
    print_result(&comm_result, &args);

    Ok(())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(
            File::open(filename)
                .map_err(|e| format!("{}: {}", filename, e))?
        )))
    }
}

// Todo:
// case-sensitiveのやつ
fn proc_lines(
    iter1: &mut Lines<Box<dyn BufRead>>,
    iter2: &mut Lines<Box<dyn BufRead>>,
    args: &Args
) -> MyResult<Vec<(usize, String)>> {
    let mut vec = Vec::new();
    let mut content1 = iter1.next();
    let mut content2 = iter2.next();

    loop {
        match (&content1, &content2) {
            (None, None) => {
                break
            },
            (None, Some(res)) => {
                let line = res.as_ref().map_err(|e| format!("line処理: {:#?}", e))?;
                vec.push((2, line.clone()));
                content2 = iter2.next()
            },
            (Some(res), None) => {
                let line = res.as_ref().map_err(|e| format!("line処理: {:#?}", e))?;
                vec.push((1, line.clone()));
                content1 = iter1.next()
            }
            (Some(res1), Some(res2)) => {
                let line1 = res1.as_ref().map_err(|e| format!("line処理: {:#?}", e))?;
                let line2 = res2.as_ref().map_err(|e| format!("line処理: {:#?}", e))?;
                let (cmp1, cmp2) = if args.insensitive {
                    (line1.to_lowercase(), line2.to_lowercase())
                } else {
                    (line1.to_string(), line2.to_string())
                };
                if cmp1 < cmp2 {
                    vec.push((1, line1.clone()));
                    content1 = iter1.next()
                } else if cmp1 > cmp2 {
                    vec.push((2, line2.clone()));
                    content2 = iter2.next()
                } else {
                    vec.push((3, line1.clone()));
                    content1 = iter1.next();
                    content2 = iter2.next()
                }
            }
        }
    }

    Ok(vec)
}

// 表示しないカラムがある場合は、左詰めにしないといけない
fn print_result(res: &Vec<(usize, String)>, args: &Args) {
    res.iter().for_each(|(i, line)| {
        match i {
            1 if args.show_col1 => {
                println!("{}", line);
            }
            2 if args.show_col2 => {
                if args.show_col1 {
                    println!("{}{}", args.delimiter, line);
                } else {
                    println!("{}", line);
                }
            }
            3 if args.show_col3 => {
                if args.show_col1 && args.show_col2 {
                    println!("{}{}{}", args.delimiter, args.delimiter, line);
                } else if args.show_col1 || args.show_col2 {
                    println!("{}{}", args.delimiter, line);
                } else {
                    println!("{}", line);
                }
            }
            _ => ()
        }
    })
}

#[test]
fn my_test() -> MyResult<()> {
    Ok(())
}
