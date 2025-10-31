use anyhow::{Context, Result};
use iota_protocol::get_socket_path;
use iota_terminal::Terminal;
use log::{info, warn};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::time::sleep;

/// Check if the server is running by attempting to connect to the socket
async fn server_is_running(socket_path: &std::path::Path) -> bool {
    use interprocess::local_socket::GenericNamespaced;
    use interprocess::local_socket::tokio::prelude::*;

    let socket_name = match socket_path.to_str() {
        Some(s) => match s.to_ns_name::<GenericNamespaced>() {
            Ok(name) => name,
            Err(_) => return false,
        },
        None => return false,
    };

    // Try to connect - if it works, server is running
    interprocess::local_socket::tokio::Stream::connect(socket_name)
        .await
        .is_ok()
}

/// Spawn the server process in the background
fn spawn_server() -> Result<()> {
    info!("Starting iota-server...");

    // Find the server binary - try multiple locations
    let server_binary = if let Ok(exe_path) = std::env::current_exe() {
        // First, try to find it in the same directory as the terminal binary
        let mut server_path = exe_path.parent().unwrap().to_path_buf();
        server_path.push("iota-server");
        if server_path.exists() {
            server_path
        } else {
            // Fall back to just "iota-server" (will search PATH)
            std::path::PathBuf::from("iota-server")
        }
    } else {
        std::path::PathBuf::from("iota-server")
    };

    Command::new(&server_binary)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .with_context(|| {
            format!(
                "Failed to spawn server process. Make sure 'iota-server' is built and in PATH.\n\
                 Tried: {:?}\n\
                 Run 'cargo build -p iota-server' to build it.",
                server_binary
            )
        })?;

    info!("Server process spawned successfully");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    // Get socket path from environment or use default
    let socket_path = get_socket_path();

    info!("Using socket path: {:?}", socket_path);

    // Check if server is already running
    if !server_is_running(&socket_path).await {
        warn!("Server not running, starting it...");
        spawn_server()?;

        // Wait for server to start (with retries)
        let max_retries = 10;
        let retry_delay = Duration::from_millis(200);

        for attempt in 1..=max_retries {
            sleep(retry_delay).await;

            if server_is_running(&socket_path).await {
                info!("Server is now running (attempt {})", attempt);
                break;
            }

            if attempt == max_retries {
                anyhow::bail!(
                    "Server failed to start after {} attempts. \
                     Check if the server binary is working: 'cargo run -p iota-server'",
                    max_retries
                );
            }

            info!(
                "Waiting for server to start (attempt {}/{})",
                attempt, max_retries
            );
        }
    } else {
        info!("Server already running");
    }

    // Connect to server
    let mut terminal = Terminal::connect(socket_path).await?;
    terminal.run().await?;

    Ok(())
}
