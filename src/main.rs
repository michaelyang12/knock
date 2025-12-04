mod args;
mod client;
mod config;

use std::io::Write;
use std::process::Command;
use std::process::Stdio;

use crate::args::Args;
use crate::client::RequestClient;
use clap::Parser;
use colored::*;

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let res = RequestClient::new(args.clone())
        .make_request()
        .await
        .expect("Error getting response");
    println!("{}", &res.bright_green());
    if !(&args.verbose) {
        copy_to_clipboard(&res).expect("Error copying to clipboard");
        // println!("{}", "result copied to clipboard!".red());
    }
}

fn copy_to_clipboard(text: &str) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "macos")]
    let cmd = "pbcopy";

    #[cfg(target_os = "linux")]
    let cmd = "wl-copy";

    let mut child = Command::new(cmd).stdin(Stdio::piped()).spawn()?;

    child.stdin.as_mut().unwrap().write_all(text.as_bytes())?;
    child.wait()?;
    Ok(())
}
