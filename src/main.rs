use std::io::{self, BufWriter};
use std::process::ExitCode;
use std::{env, fs};

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: brainrust <file.bf>");
        return ExitCode::from(2);
    };
    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("brainrust: cannot read {path}: {e}");
            return ExitCode::from(2);
        }
    };
    let output = BufWriter::new(io::stdout().lock());
    match brainrust::run(&source, io::stdin().lock(), output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("brainrust: {e}");
            ExitCode::FAILURE
        }
    }
}
