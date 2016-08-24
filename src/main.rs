#[macro_use] extern crate itertools;

use std::env;
use std::process::exit;
use std::io::{Write,BufRead,BufReader};
use std::fs::File;

use itertools::Itertools;
use itertools::EitherOrBoth::{Both, Right, Left};


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

const MAX_BATCH_SIZE: usize = 100;


pub fn main_i() -> i32 {
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    if args.len() != 3 {
        println_stderr!("Usage: {} file1 file2", program);
        return 2;
    }

    let file1 = BufReader::new(File::open(args[1].clone()).expect(&format!("Could not open {:?}", args[1])));
    let file2 = BufReader::new(File::open(args[2].clone()).expect(&format!("Could not open {:?}", args[2])));

    let differences = file1.lines().zip_longest(file2.lines()).enumerate().filter_map( |(i, zr)| {
        if i % 100000 == 0 {
            println_stderr!("# {}", i);
        }
        match zr {
            Both(l, r) => {
                let lhs = l.unwrap();
                let rhs = r.unwrap();
                if lhs != rhs {
                    Some(Difference::new(i, Some(lhs.to_owned()), Some(rhs.to_owned())))
                } else {
                    None
                }
            },
            Left(l) => {
                Some(Difference::new(i, Some(l.unwrap()), None))
            },
            Right(r) => {
                Some(Difference::new(i, None, Some(r.unwrap())))
            }
        }
    });

    // Group the diffs into batches of consecutive lines
    let difference_batches = differences.peekable().batching(|it| {
        let mut resp = vec!();
        match it.next() {
            None => None,
            Some(first) => {
                let first_line = first.line;
                let mut cur_line = first.line;
                resp.push(first);
                while resp.len() < MAX_BATCH_SIZE {
                    // it would be clearer here if we could call the it.next() inside the
                    // it.peek(), but the borrow checker disallows it, so we break instead
                    //
                    // NOTE: it.peek() will appear to block until the next diff item, and we will
                    // just be sitting here holding the bag on the current item. We could probably
                    // work around that by having the differences iterator generator above emit
                    // a sigil every once in a while saying that it hasn't found any differences.
                    if let Some(ref diff) = it.peek() {
                        if diff.line != cur_line + 1 {
                            break;
                        }
                    } else {
                        break;
                    }
                    let next_line = it.next().unwrap();
                    cur_line  = next_line.line;
                    resp.push(next_line);
                }
                Some((first_line, resp))
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

    return 0;
}

pub fn main() {
    exit(main_i());
}
