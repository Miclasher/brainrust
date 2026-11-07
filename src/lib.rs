//! A simple Brainfuck interpreter.

use std::fmt;
use std::io::{self, Read, Write};

/// Number of cells on the tape.
pub const TAPE_SIZE: usize = 30_000;

#[derive(Debug)]
pub enum Error {
    /// A `]` at the given source offset has no matching `[`.
    UnmatchedClose(usize),
    /// A `[` at the given source offset has no matching `]`.
    UnmatchedOpen(usize),
    /// The data pointer moved off either end of the tape.
    PointerOutOfBounds,
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::UnmatchedClose(i) => write!(f, "unmatched ']' at offset {i}"),
            Error::UnmatchedOpen(i) => write!(f, "unmatched '[' at offset {i}"),
            Error::PointerOutOfBounds => write!(f, "data pointer out of bounds"),
            Error::Io(e) => write!(f, "I/O error: {e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::Io(e)
    }
}

/// Strips non-command characters and computes bracket jump targets.
fn compile(source: &str) -> Result<(Vec<u8>, Vec<usize>), Error> {
    let mut code = Vec::new();
    let mut offsets = Vec::new();
    for (i, b) in source.bytes().enumerate() {
        if b"+-<>[].,".contains(&b) {
            code.push(b);
            offsets.push(i);
        }
    }

    let mut jumps = vec![0; code.len()];
    let mut stack = Vec::new();
    for (pc, &op) in code.iter().enumerate() {
        match op {
            b'[' => stack.push(pc),
            b']' => {
                let open = stack.pop().ok_or(Error::UnmatchedClose(offsets[pc]))?;
                jumps[open] = pc;
                jumps[pc] = open;
            }
            _ => {}
        }
    }
    if let Some(open) = stack.pop() {
        return Err(Error::UnmatchedOpen(offsets[open]));
    }
    Ok((code, jumps))
}

/// Runs `source`, reading from `input` and writing to `output`.
///
/// Cells are wrapping bytes; `,` stores 0 on end of input.
pub fn run<R: Read, W: Write>(source: &str, mut input: R, mut output: W) -> Result<(), Error> {
    let (code, jumps) = compile(source)?;
    let mut tape = vec![0u8; TAPE_SIZE];
    let mut ptr = 0usize;
    let mut pc = 0usize;

    while pc < code.len() {
        match code[pc] {
            b'+' => tape[ptr] = tape[ptr].wrapping_add(1),
            b'-' => tape[ptr] = tape[ptr].wrapping_sub(1),
            b'>' => {
                ptr += 1;
                if ptr >= TAPE_SIZE {
                    return Err(Error::PointerOutOfBounds);
                }
            }
            b'<' => ptr = ptr.checked_sub(1).ok_or(Error::PointerOutOfBounds)?,
            b'.' => output.write_all(&tape[ptr..=ptr])?,
            b',' => {
                let mut buf = [0u8; 1];
                tape[ptr] = match input.read(&mut buf)? {
                    0 => 0,
                    _ => buf[0],
                };
            }
            b'[' if tape[ptr] == 0 => pc = jumps[pc],
            b']' if tape[ptr] != 0 => pc = jumps[pc],
            _ => {}
        }
        pc += 1;
    }
    output.flush()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn exec(src: &str, input: &str) -> Result<String, Error> {
        let mut out = Vec::new();
        run(src, input.as_bytes(), &mut out)?;
        Ok(String::from_utf8_lossy(&out).into_owned())
    }

    #[test]
    fn hello_world() {
        let src = "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.";
        assert_eq!(exec(src, "").unwrap(), "Hello World!\n");
    }

    #[test]
    fn increment_and_output() {
        assert_eq!(exec(&format!("{}.", "+".repeat(65)), "").unwrap(), "A");
    }

    #[test]
    fn echo_input() {
        assert_eq!(exec(",[.,]", "abc").unwrap(), "abc");
    }

    #[test]
    fn eof_reads_zero() {
        assert_eq!(exec("+,.", "").unwrap(), "\0");
    }

    #[test]
    fn cell_wraps_both_ways() {
        let mut out = Vec::new();
        run("-.+.", io::empty(), &mut out).unwrap();
        assert_eq!(out, [255, 0]);
    }

    #[test]
    fn skips_loop_when_zero() {
        assert_eq!(exec("[+++.]", "").unwrap(), "");
    }

    #[test]
    fn nested_loops_multiply() {
        // 3 * 4 * 5 + 5 = 65 -> 'A'
        assert_eq!(exec("+++[>++++[>+++++<-]<-]>>+++++.", "").unwrap(), "A");
    }

    #[test]
    fn ignores_comments() {
        assert_eq!(exec("++ hi ++[->+++++++++++++++<]>+++++ . end", "").unwrap(), "A");
    }

    #[test]
    fn unmatched_close() {
        assert!(matches!(exec("+]", ""), Err(Error::UnmatchedClose(1))));
    }

    #[test]
    fn unmatched_open() {
        assert!(matches!(exec("+[", ""), Err(Error::UnmatchedOpen(1))));
    }

    #[test]
    fn pointer_underflow() {
        assert!(matches!(exec("<", ""), Err(Error::PointerOutOfBounds)));
    }

    #[test]
    fn pointer_overflow() {
        let src = ">".repeat(TAPE_SIZE);
        assert!(matches!(exec(&src, ""), Err(Error::PointerOutOfBounds)));
        let ok = ">".repeat(TAPE_SIZE - 1);
        assert!(exec(&ok, "").is_ok());
    }
}
