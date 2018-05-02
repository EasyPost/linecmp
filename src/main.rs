extern crate itertools;
extern crate clap;

use std::process::exit;
use std::io::{self,BufRead,BufReader,Write};
use std::fs::File;
use std::fmt;
use std::error::Error;

use itertools::Itertools;
use itertools::EitherOrBoth::{Both, Right, Left};
use clap::Arg;


#[derive(Debug)]
struct Difference {
    line: usize,
    lhs: Option<String>,
    rhs: Option<String>,
}


impl Difference {
    fn new(i: usize, lhs: Option<String>, rhs: Option<String>) -> Self {
        Difference {
            line: i + 1,
            lhs,
            rhs
        }
    }
}

enum DiffItem {
    Difference(Difference),
    NoDifference
}

const MAX_BATCH_SIZE: usize = 100;


#[derive(Debug)]
enum MainError {
    FileOpenError { filename: String, error: io::Error },
    WriteError(io::Error),
}

impl fmt::Display for MainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MainError::FileOpenError { ref filename, ref error } => {
                write!(f, "Error opening {}: {}", filename, error)
            }
            _ => <Self as fmt::Debug>::fmt(self, f)
        }
    }
}


impl Error for MainError {
    fn description(&self) -> &'static str {
        "error"
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            MainError::FileOpenError { error: ref e, .. } => Some(e),
            MainError::WriteError(ref e) => Some(e),
        }
    }
}


fn open(path: &str) -> Result<BufReader<File>, io::Error> {
    Ok(BufReader::with_capacity(
        1 << 19,
        File::open(
            path
        )?
    ))
}


fn write_difference_batches<W, I>(mut target: W, difference_batches: I) -> Result<(), io::Error>
    where W: Write, I: Iterator<Item=(usize, Vec<Difference>)> {
    for (first_line, diff) in difference_batches {
        writeln!(target, "line {}", first_line)?;
        for di in &diff {
            match di.lhs {
                Some(ref l) => writeln!(target, "< {}", l)?,
                None => writeln!(target, "< [missing]")?,
            };
        }
        for di in diff {
            match di.rhs {
                Some(ref l) => writeln!(target, "> {}", l)?,
                None => writeln!(target, "> [missing]")?,
            };
        }
    }
    Ok(())
}


fn main_i() -> Result<i32, MainError> {
    let matches = clap::App::new(env!("CARGO_PKG_NAME"))
                            .version(env!("CARGO_PKG_VERSION"))
                            .author("EasyPost <oss@easypost.com>")
                            .about(env!("CARGO_PKG_DESCRIPTION"))
                            .arg(Arg::with_name("file1")
                                     .required(true)
                                     .help("Path to LHS file operand"))
                            .arg(Arg::with_name("file2")
                                     .required(true)
                                     .help("Path to RHS file operand"))
                            .get_matches();

    let path1 = matches.value_of("file1").unwrap();
    let path2 = matches.value_of("file2").unwrap();
    let file1 = open(&path1).map_err(|e| MainError::FileOpenError { filename: path1.to_owned(), error: e })?;
    let file2 = open(&path2).map_err(|e| MainError::FileOpenError { filename: path2.to_owned(), error: e })?;

    let differences = file1.lines().zip_longest(file2.lines()).enumerate().filter_map( |(i, zr)| {
        if i % 100_000 == 0 {
            eprintln!("# {}", i);
        }
        match zr {
            Both(l, r) => {
                let lhs = l.unwrap();
                let rhs = r.unwrap();
                if lhs != rhs {
                    Some(DiffItem::Difference(Difference::new(i, Some(lhs.to_owned()), Some(rhs.to_owned()))))
                } else if i % 10000 == 0 {
                    // emit a NoDifference chunk every few thousand lines to prevent it.peek()
                    // from blocking for a really long time in between differences
                    Some(DiffItem::NoDifference)
                } else {
                    // Most of the time, though, just have filter_map elide those
                    None
                }
            },
            Left(l) => {
                Some(DiffItem::Difference(Difference::new(i, Some(l.unwrap()), None)))
            },
            Right(r) => {
                Some(DiffItem::Difference(Difference::new(i, None, Some(r.unwrap()))))
            }
        }
    });

    // Group the diffs into batches of consecutive lines
    let difference_batches = differences.peekable().batching(|it| {
        let mut resp = vec!();
        loop {
            match it.next() {
                None => { return None; },
                Some(DiffItem::NoDifference) => { continue },
                Some(DiffItem::Difference(first)) => {
                    let first_line = first.line;
                    let mut cur_line = first.line;
                    resp.push(first);
                    while resp.len() < MAX_BATCH_SIZE {
                        // it would be clearer here if we could call the it.next() inside the
                        // it.peek(), but the borrow checker disallows it, so we break instead
                        match it.peek() {
                            Some(&DiffItem::NoDifference) => {
                                break;
                            }
                            Some(&DiffItem::Difference(ref diff)) => {
                                if diff.line != cur_line + 1 {
                                    break;
                                }
                            },
                            None => {
                                break;
                            }
                        };
                        if let DiffItem::Difference(next_line) = it.next().unwrap() {
                            cur_line  = next_line.line;
                            resp.push(next_line);
                        }
                    }
                    return Some((first_line, resp));
                }
            }
        }
    });

    let stdout = io::stdout();
    write_difference_batches(stdout.lock(), difference_batches).map_err(MainError::WriteError)?;

    Ok(0)
}

pub fn main() {
    match main_i() {
        Ok(i) => exit(i),
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    }
}
