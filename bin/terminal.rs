use anyhow::Result;
use iota::{editor::Editor, terminal::Terminal};
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    // Open file if provided, otherwise create empty editor
    let editor = if args.len() > 1 {
        Editor::with_file(&args[1])?
    } else {
        Editor::new()
    };

    let mut frontend = Terminal::new(editor)?;
    frontend.run()?;

    Ok(())
}
