use std::io::{self, BufRead};

pub fn ask(mesg: &str) -> bool {
    println!("{}", mesg);
    let stdin = io::stdin();
    let line = stdin
        .lock()
        .lines()
        .next()
        .expect("no user input")
        .expect("could not read input line");

    if line.to_lowercase() == "y" || line.to_lowercase() == "yes" {
        return true;
    }
    return false;
}
