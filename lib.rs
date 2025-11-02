use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(version, about = "The iota text editor")]
pub struct Args {
    #[arg(help = "Files to open.")]
    files: Vec<String>,

    #[arg(
        long,
        default_value = "false",
        help = "Whether or not to start the server. If the client closes, the daemon will continue running."
    )]
    server: bool,
}
