use std::process::Command;

use clap::Parser;
use iota::Args;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("args: {:?}", args);

    Ok(())
}
