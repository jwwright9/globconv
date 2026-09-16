mod convert;

use std::env;
use std::io::{self, BufRead, Write};
use std::process::ExitCode;

fn usage(prog: &str) -> String {
    format!(
        "usage:\n  {prog} to-regex <glob-pattern>\n  {prog} to-glob <regex-pattern>\n  \
         {prog} to-regex  (reads patterns from stdin, one per line)\n  \
         {prog} to-glob   (reads patterns from stdin, one per line)\n\n\
         examples:\n  {prog} to-regex '*.txt'\n  {prog} to-glob '^.*\\.txt$'",
        prog = prog
    )
}

fn convert_one(mode: &str, pattern: &str) -> Result<String, String> {
    match mode {
        "to-regex" => Ok(convert::glob_to_regex(pattern)),
        "to-glob" => convert::regex_to_glob(pattern),
        _ => Err(format!("unknown mode '{}'", mode)),
    }
}

fn run_batch(mode: &str) -> ExitCode {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();
    let mut had_error = false;

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                eprintln!("error: reading stdin: {}", e);
                return ExitCode::FAILURE;
            }
        };
        if line.is_empty() {
            continue;
        }
        match convert_one(mode, &line) {
            Ok(result) => {
                let _ = writeln!(out, "{}", result);
            }
            Err(e) => {
                had_error = true;
                eprintln!("error: {}: {}", line, e);
            }
        }
    }

    if had_error {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let prog = args
        .first()
        .map(|s| s.as_str())
        .unwrap_or("globconv")
        .to_string();

    match args.len() {
        2 => {
            let mode = args[1].as_str();
            if mode != "to-regex" && mode != "to-glob" {
                eprintln!("{}", usage(&prog));
                return ExitCode::FAILURE;
            }
            run_batch(mode)
        }
        3 => {
            let mode = args[1].as_str();
            if mode != "to-regex" && mode != "to-glob" {
                eprintln!("{}", usage(&prog));
                return ExitCode::FAILURE;
            }
            let pattern = &args[2];
            match convert_one(mode, pattern) {
                Ok(result) => {
                    println!("{}", result);
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: {}", e);
                    ExitCode::FAILURE
                }
            }
        }
        _ => {
            eprintln!("{}", usage(&prog));
            ExitCode::FAILURE
        }
    }
}
