use std::{error::Error, str::FromStr};

use ansi_term::Style;
use chrono::{Datelike, Local, NaiveDate, Weekday};
use clap::Parser;
use itertools::Itertools;
use regex::RegexBuilder;

type MyResult<T> = Result<T, Box<dyn Error>>;

// ----------------------------------------------------------------------
#[derive(Debug, Parser)]
pub struct Args {
    /// year
    // 何年のカレンダーを表示するか
    // 指定されない場合は今年が設定される
    #[arg(short, long)]
    year: Option<String>,

    /// month
    //　何月のカレンダーを表示するか
    // 指定されない場合は12月分すべてが表示される
    #[arg(short, long)]
    month: Option<String>,
}

// ----------------------------------------------------------------------
impl Args {
    fn to_config(&self) -> MyResult<Config> {
        let today = Local::now();

        let year = self
            .year
            .as_ref()
            .map_or(Ok(today.year()), |y| parse_year(&y))?;

        let month = self.month.as_ref().map(|m| parse_month(&m)).transpose()?;

        Ok(Config { year, month })
    }
}

// ----------------------------------------------------------------------
pub fn get_config() -> MyResult<Config> {
    Args::parse().to_config()
}

// ----------------------------------------------------------------------
#[derive(Debug)]
pub struct Config {
    year: i32,
    month: Option<u32>,
}

// ----------------------------------------------------------------------
pub fn run(config: Config) -> MyResult<()> {
    let today = Local::now().date_naive();

    match config.month {
        None => {
            // year全体を表示する
            //　各月のtitleにはyearは表示しない
            let header = format!(
                "{}{}{}",
                " ".repeat(28),
                config.year.to_string(),
                " ".repeat(66 - 28 - config.year.to_string().len())
            );
            println!("{}", header);
            let body = (1..=12)
                .map(|month| format_month(config.year, month, false, today))
                .chunks(3)
                .into_iter()
                .map(|vecs| {
                    vecs.into_iter()
                        .reduce(|acc, row| {
                            acc.into_iter()
                                .zip(row.into_iter())
                                .map(|(a, b)| a + &b)
                                .collect_vec()
                        })
                        .unwrap()
                })
                .collect::<Vec<_>>();
            body.iter().for_each(|three_month| {
                three_month.iter().for_each(|line| println!("{}", line));
                println!("")
            });
        }
        Some(month) => {
            // 指定された月だけを表示する
            // titleにyearも表示する
            let calendar = format_month(config.year, month, true, today);
            calendar.iter().for_each(|line| println!("{}", line));
        }
    }
    Ok(())
}

// ----------------------------------------------------------------------
fn parse_year(year: &str) -> MyResult<i32> {
    match parse_int::<i32>(year) {
        Ok(year) if 1 <= year && year <= 9999 => Ok(year),
        Ok(year) => Err(From::from(format!(
            "year \"{}\" not in the range 1 through 9999",
            year
        ))),
        Err(e) => Err(e),
    }
}

// ----------------------------------------------------------------------
fn parse_month(month: &str) -> MyResult<u32> {
    match parse_int::<u32>(month) {
        Ok(month) if 1 <= month && month <= 12 => Ok(month),
        Ok(month) => Err(From::from(format!(
            "month \"{}\" not in the range 1 through 12",
            month
        ))),
        Err(_e) => {
            let re = RegexBuilder::new(&format!("^{}", month))
                .case_insensitive(true)
                .build()
                .map_err(|_| format!("Invalid pattern \"{}\"", month))?;

            let filtered_month = MONTHS
                .iter()
                .enumerate()
                .filter(|(_, month)| re.is_match(month))
                .collect::<Vec<_>>();

            if filtered_month.len() == 1 {
                Ok((filtered_month[0].0 + 1) as u32)
            } else {
                Err(format!("Invalid month \"{}\"", month).into())
            }
        }
    }
}

// ----------------------------------------------------------------------
const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

// ----------------------------------------------------------------------
fn parse_int<T: FromStr>(val: &str) -> MyResult<T> {
    val.parse()
        .map_err(|_| format!("Invalid integer \"{}\"", val).into())
}

// ----------------------------------------------------------------------
fn format_month(year: i32, month: u32, print_year: bool, today: NaiveDate) -> Vec<String> {
    // Todo: ここから
    // year, month のカレンダーを表示する。today が含まれるならそこだけ反転する
    // 必ず 8行22列
    // year monthに関しては、21列の真ん中にする（長さが奇数の場合は左にずれる）
    let title = if print_year {
        format!("{} {}", MONTHS[(month - 1) as usize], year)
    } else {
        format!("{}", MONTHS[(month - 1) as usize])
    };
    // title は 1 行目の 11-(2/len) 列から始まる
    // 11-(len/2)-1 個の " " + title + 13-len+(len/2) 個の " "
    let len = title.chars().count();
    let top_line = [
        " ".repeat(11 - ((len + 1) / 2) - 1),
        title,
        " ".repeat(12 + ((len + 1) / 2) - len),
    ]
    .join("");
    let week = String::from("Su Mo Tu We Th Fr Sa  ");

    // 1~最終日までループしてVec<String> を作ってく
    // 1の時、それまでの曜日に空きがあればその分を空白で埋める
    // 最終日の時、その後の曜日に空きがあればその分を空白で埋める
    // 全ての日にちにおいて、土曜日だけ特殊処理が入る
    let mut days = vec![];
    let mut line = String::from("");
    let last_day = last_day_in_month(year, month).unwrap();

    for i in 1..=(last_day.day() as usize) {
        let date = NaiveDate::from_ymd_opt(year, month, i as u32).unwrap();
        let weekday = date.weekday();
        if i == 1 {
            let offset = weekday.num_days_from_sunday();
            line = "   ".repeat(offset as usize);
            line = format!("{}{} ", line, print_day(today, year, month, i));
            if weekday == Weekday::Sat {
                line = format!("{} ", line);
                days.push(line);
                line = "".to_string();
            }
        } else if i == last_day.day() as usize {
            line = format!("{}{} ", line, print_day(today, year, month, i));
            let offset = 6 - weekday.num_days_from_sunday();
            line = format!("{}{} ", line, "   ".repeat(offset as usize));
            days.push(line);
            line = "".to_string()
        } else {
            line = format!("{}{} ", line, print_day(today, year, month, i));
            if weekday == Weekday::Sat {
                line = format!("{} ", line);
                days.push(line);
                line = "".to_string()
            }
        }
    }

    if days.len() < 6 {
        // 長さが６でないなら5であるしかなく、6に合わせるために空行を加える
        days.push(" ".repeat(22))
    }

    std::iter::once(top_line)
        .chain(std::iter::once(week))
        .chain(days.into_iter())
        .collect()
}

// ----------------------------------------------------------------------
fn last_day_in_month(year: i32, month: u32) -> MyResult<NaiveDate> {
    let (y, m) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };

    Ok(NaiveDate::from_ymd_opt(y, m, 1)
        .ok_or(format!("err: NaiveDateの取得 year: {}, month: {}", year, month).into())
        .and_then(|date| date.pred_opt().ok_or(format!("err: NaiveDateの前日の取得")))?)
}

// ----------------------------------------------------------------------
fn print_day(today: NaiveDate, year: i32, month: u32, day: usize) -> String {
    let style = Style::new().reverse();
    let num_str = day.to_string();

    if today.year() == year && today.month() == month && today.day() == day as u32 {
        if num_str.len() == 1 {
            style.paint(format!(" {}", num_str)).to_string()
        } else {
            style.paint(num_str).to_string()
        }
    } else {
        format!("{:>2}", day)
    }
}

// ----------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate};

    use crate::{format_month, last_day_in_month, parse_month, parse_year};

    use super::parse_int;

    #[test]
    fn test_parse_int() {
        let res = parse_int::<usize>("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1usize);

        let res = parse_int::<i32>("-1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), -1i32);

        let res = parse_int::<i64>("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "Invalid integer \"foo\"");
    }

    #[test]
    fn test_parse_year() {
        let res = parse_year("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1i32);

        let res = parse_year("9999");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 9999i32);

        let res = parse_year("0");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "year \"0\" not in the range 1 through 9999"
        );

        let res = parse_year("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "Invalid integer \"foo\"");
    }

    #[test]
    fn test_parse_month() {
        let res = parse_month("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1);

        let res = parse_month("12");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 12);

        let res = parse_month("0");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            format!("month \"0\" not in the range 1 through 12")
        );

        let res = parse_month("13");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            format!("month \"13\" not in the range 1 through 12")
        );

        let res = parse_month("jan");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1);

        let res = parse_month("JaN");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), 1);

        let res = parse_month("ju");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            format!("Invalid month \"ju\"")
        );
    }

    #[test]
    fn test_last_day_in_month() {
        let res = last_day_in_month(2020, 2);
        assert!(res.is_ok());
        assert_eq!(29, res.unwrap().day());
    }

    #[test]
    fn test_format_month() {
        let today = NaiveDate::from_ymd_opt(0, 1, 1).unwrap();
        let leap_february = vec![
            "   February 2020      ",
            "Su Mo Tu We Th Fr Sa  ",
            "                   1  ",
            " 2  3  4  5  6  7  8  ",
            " 9 10 11 12 13 14 15  ",
            "16 17 18 19 20 21 22  ",
            "23 24 25 26 27 28 29  ",
            "                      ",
        ];
        assert_eq!(format_month(2020, 2, true, today), leap_february);

        let may = vec![
            "        May           ",
            "Su Mo Tu We Th Fr Sa  ",
            "                1  2  ",
            " 3  4  5  6  7  8  9  ",
            "10 11 12 13 14 15 16  ",
            "17 18 19 20 21 22 23  ",
            "24 25 26 27 28 29 30  ",
            "31                    ",
        ];
        assert_eq!(format_month(2020, 5, false, today), may);

        let april_hl = vec![
            "     April 2021       ",
            "Su Mo Tu We Th Fr Sa  ",
            "             1  2  3  ",
            " 4  5  6 \u{1b}[7m 7\u{1b}[0m  8  9 10  ",
            "11 12 13 14 15 16 17  ",
            "18 19 20 21 22 23 24  ",
            "25 26 27 28 29 30     ",
            "                      ",
        ];
        let today = NaiveDate::from_ymd_opt(2021, 4, 7).unwrap();
        assert_eq!(format_month(2021, 4, true, today), april_hl);
    }
}
