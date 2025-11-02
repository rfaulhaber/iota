use anyhow::Result;
use iota_protocol::get_socket_path;
use iota_terminal::Terminal;
use log::info;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Get socket path from environment or use default
    let socket_path = get_socket_path();

    info!("Using socket path: {:?}", socket_path);

    // Connect to server
    let mut terminal = Terminal::connect(socket_path).await?;
    terminal.run().await?;

    Ok(())
}
