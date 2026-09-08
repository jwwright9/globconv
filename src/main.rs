mod convert;

use std::env;
use std::process::ExitCode;

fn usage(prog: &str) -> String {
    format!(
        "usage:\n  {prog} to-regex <glob-pattern>\n  {prog} to-glob <regex-pattern>\n\n\
         examples:\n  {prog} to-regex '*.txt'\n  {prog} to-glob '^.*\\.txt$'",
        prog = prog
    )
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let prog = args
        .first()
        .map(|s| s.as_str())
        .unwrap_or("globconv")
        .to_string();

    if args.len() != 3 {
        eprintln!("{}", usage(&prog));
        return ExitCode::FAILURE;
    }

    let mode = args[1].as_str();
    let pattern = &args[2];

    match mode {
        "to-regex" => {
            println!("{}", convert::glob_to_regex(pattern));
            ExitCode::SUCCESS
        }
        "to-glob" => match convert::regex_to_glob(pattern) {
            Ok(glob) => {
                println!("{}", glob);
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {}", e);
                ExitCode::FAILURE
            }
        },
        _ => {
            eprintln!("{}", usage(&prog));
            ExitCode::FAILURE
        }
    }
}
