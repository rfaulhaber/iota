use anyhow::Result;
use iota::{editor::Editor, terminal::Terminal};
use std::env;

fn main() -> Result<()> {
    // let args: Vec<String> = env::args().collect();

    // Create the headless editor
    let editor = Editor::new();

    // Open file if provided
    // if args.len() > 1 {
    //     editor.open_file(&args[1])?;
    // }

    // Create and run the terminal frontend
    let mut frontend = Terminal::new(editor)?;
    frontend.run()?;

    Ok(())
}
