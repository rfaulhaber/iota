use std::io::{self, Read, Write};
use std::process::exit;

fn main() {
    read_stdin().unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(1);
    });
}

fn read_stdin() -> io::Result<()> {
    println!("started");

    loop {
        let mut buf = String::new();

        io::stdin()
            .read_line(&mut buf)
            .expect("could not read from stdin");

        eprintln!("iota: received input: {}", buf);
        println!("{}", buf);
    }

    Ok(())
}
