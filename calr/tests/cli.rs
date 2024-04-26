use assert_cmd::Command;
use predicates::prelude::*;
use std::{error::Error, fs};

type TestResult = Result<(), Box<dyn Error>>;

const PRG: &str = "calr";

// --------------------------------------------------
#[test]
fn dies_year_0() -> TestResult {
    Command::cargo_bin(PRG)?
        .args(&["-y", "0"])
        .assert()
        .failure()
        .stderr("year \"0\" not in the range 1 through 9999\n");
    Ok(())
}

// --------------------------------------------------
#[test]
fn dies_year_13() -> TestResult {
    Command::cargo_bin(PRG)?
        .args(&["-y", "10000"])
        .assert()
        .failure()
        .stderr("year \"10000\" not in the range 1 through 9999\n");
    Ok(())
}

// --------------------------------------------------
#[test]
fn dies_invalid_year() -> TestResult {
    Command::cargo_bin(PRG)?
        .args(&["-y", "foo"])
        .assert()
        .failure()
        .stderr("Invalid integer \"foo\"\n");
    Ok(())
}

// --------------------------------------------------
#[test]
fn dies_month_0() -> TestResult {
    Command::cargo_bin(PRG)?
        .args(&["-m", "0"])
        .assert()
        .failure()
        .stderr("month \"0\" not in the range 1 through 12\n");
    Ok(())
}

// --------------------------------------------------
#[test]
fn dies_month_13() -> TestResult {
    Command::cargo_bin(PRG)?
        .args(&["-m", "13"])
        .assert()
        .failure()
        .stderr("month \"13\" not in the range 1 through 12\n");
    Ok(())
}

// --------------------------------------------------
#[test]
fn dies_invalid_month() -> TestResult {
    Command::cargo_bin(PRG)?
        .args(&["-m", "foo"])
        .assert()
        .failure()
        .stderr("Invalid month \"foo\"\n");
    Ok(())
}

// --------------------------------------------------
#[test]
fn month_num() -> TestResult {
    let expected = &[
        ("1", "January"),
        ("2", "February"),
        ("3", "March"),
        ("4", "April"),
        ("5", "May"),
        ("6", "June"),
        ("7", "July"),
        ("8", "August"),
        ("9", "September"),
        ("10", "October"),
        ("11", "November"),
        ("12", "December"),
    ];

    for (num, month) in expected {
        Command::cargo_bin(PRG)?
            .args(&["-m", num])
            .assert()
            .success()
            .stdout(predicates::str::contains(month.to_string()));
    }
    Ok(())
}

// --------------------------------------------------
#[test]
fn partial_month() -> TestResult {
    let expected = &[
        ("ja", "January"),
        ("f", "February"),
        ("mar", "March"),
        ("ap", "April"),
        ("may", "May"),
        ("jun", "June"),
        ("jul", "July"),
        ("au", "August"),
        ("s", "September"),
        ("n", "November"),
        ("d", "December"),
    ];

    for (arg, month) in expected {
        Command::cargo_bin(PRG)?
            .args(&["-m", arg])
            .assert()
            .success()
            .stdout(predicates::str::contains(month.to_string()));
    }
    Ok(())
}

// --------------------------------------------------
fn run(args: &[&str], expected_file: &str) -> TestResult {
    let expected = fs::read_to_string(expected_file)?;
    Command::cargo_bin(PRG)?
        .args(args)
        .assert()
        .success()
        .stdout(expected);
    Ok(())
}

// --------------------------------------------------
#[test]
fn no_args() -> TestResult {
    let cmd = Command::cargo_bin(PRG)?.assert().success();
    let out = cmd.get_output();
    let stdout = String::from_utf8(out.stdout.clone())?;
    let lines: Vec<_> = stdout.split("\n").collect();
    assert_eq!(lines.len(), 38);
    assert_eq!(lines[0].len(), 66);
    Ok(())
}

// --------------------------------------------------
#[test]
fn test_2_2020_leap_year() -> TestResult {
    run(&["-m", "2", "-y", "2020"], "tests/expected/2-2020.txt")
}

// --------------------------------------------------
#[test]
fn test_4_2020() -> TestResult {
    run(&["-m", "4", "-y", "2020"], "tests/expected/4-2020.txt")
}

// --------------------------------------------------
#[test]
fn test_april_2020() -> TestResult {
    run(&["-y", "2020", "-m", "april"], "tests/expected/4-2020.txt")
}
