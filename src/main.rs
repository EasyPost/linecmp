#[macro_use] extern crate itertools;

use std::env;
use std::process::exit;
use std::io::{Write,BufRead,BufReader};
use std::fs::File;
use std::sync::mpsc::sync_channel;
use std::thread;

use itertools::Itertools;
use itertools::EitherOrBoth::{self, Both, Right, Left};


macro_rules! println_stderr(
    ($($arg:tt)*) => (
        match writeln!(&mut ::std::io::stderr(), $($arg)* ) {
            Ok(_) => {},
            Err(x) => panic!("Unable to write to stderr: {}", x),
        }
    )
);


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
            lhs: lhs,
            rhs: rhs
        }
    }
}

enum DiffItem {
    Difference(Difference),
    NoDifference
}

const MAX_BATCH_SIZE: usize = 100;
const READ_CHUNK_SIZE: usize = 1000;
const READ_BUF_SIZE: usize = 524288;


fn open(path: &str) -> BufReader<File> {
    BufReader::with_capacity(
        READ_BUF_SIZE,
        File::open(
            path
        ).expect(&format!("Could not open {:?}", path))
    )
}


pub fn main_i() -> i32 {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    if args.len() != 3 {
        println_stderr!("Usage: {} file1 file2", program);
        return 2;
    }

    let (lines_tx, lines_rx) = sync_channel(100);

    let source_handle = thread::spawn ( move|| {
        // Read the files (and find newlines) in a separate thread
        let file1 = open(&args[1]);
        let file2 = open(&args[2]);

        // Chunk up unprocessed lines and send them over a channel to the main thread
        for chunk in &file1.lines().zip_longest(file2.lines()).enumerate().chunks_lazy(READ_CHUNK_SIZE) {
            // TODO: Why is every item in .lines() a result? Just unwrap them for now, but maybe do
            // something saner later?
            let items: Vec<(usize, EitherOrBoth<String, String>)> = chunk.into_iter().map( |(l,it)| {
                (l, match it {
                    Both(l, r) => Both(l.unwrap(), r.unwrap()),
                    Left(l) => Left(l.unwrap()),
                    Right(r) => Right(r.unwrap())
                })
            }).collect();
            lines_tx.send(items).unwrap();
        }
    });

    // Walk through the lines and pull out those that don't match, creating an iterator
    // of Differences (wrapped up in DiffItem enums, with some DiffItem::NoDifference heartbeats
    // interspersed
    let differences = lines_rx.iter().flatten().filter_map(|(i, zr)| {
        if i % 100000 == 0 {
            println_stderr!("# {}", i);
        }
        match zr {
            Both(lhs, rhs) => {
                if lhs != rhs {
                    Some(DiffItem::Difference(Difference::new(i, Some(lhs.to_owned()), Some(rhs.to_owned()))))
                } else {
                    if i % 10000 == 0 {
                        // emit a NoDifference chunk every few thousand lines to prevent it.peek()
                        // from blocking for a really long time in between differences
                        Some(DiffItem::NoDifference)
                    } else {
                        // Most of the time, though, just have filter_map elide those
                        None
                    }
                }
            },
            Left(l) => {
                Some(DiffItem::Difference(Difference::new(i, Some(l), None)))
            },
            Right(r) => {
                Some(DiffItem::Difference(Difference::new(i, None, Some(r))))
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

    for (first_line, diff) in difference_batches {
        println!("line {}", first_line);
        for di in diff.iter() {
            match di.lhs {
                Some(ref l) => println!("< {}", l),
                None => println!("< [missing]")
            }
        }
        for di in diff.iter() {
            match di.rhs {
                Some(ref l) => println!("> {}", l),
                None => println!("> [missing]")
            }
        }
    }

    source_handle.join().unwrap();

    return 0;
}

pub fn main() {
    exit(main_i());
}
