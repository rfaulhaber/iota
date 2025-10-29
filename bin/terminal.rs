use iota_editor::Editor;
use iota_terminal::Terminal;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let args: Vec<String> = std::env::args().collect();

    let editor = if args.len() > 1 {
        Editor::with_file(&args[1]).await?
    } else {
        Editor::new()
    };

    let mut frontend = Terminal::new(editor)?;
    frontend.run().await?;

    Ok(())
}
