use std::io::{self, Read, Write};
use std::process::exit;

// TODO: create objects for editor state, view requests like: MoveCursorUp, etc.

struct EditorState {
    current_line: u128,
    current_column: u128,
}

fn main() {
    read_stdin().unwrap_or_else(|e| {
        eprintln!("{}", e);
        exit(1);
    });
}

fn read_stdin() -> io::Result<()> {
    println!("starting read loop");

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
