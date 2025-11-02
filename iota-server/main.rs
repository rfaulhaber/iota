use anyhow::Result;
use iota_protocol::get_socket_path;
use iota_server::Server;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let pid = std::process::id();
    println!("{}", pid);

    // Get socket path from environment or use default
    let socket_path = get_socket_path();

    let server = Server::local(socket_path).await?;
    server.run().await?;

    Ok(())
}
