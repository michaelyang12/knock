use colored::Colorize;
use std::process::{Command, ExitCode};

const REPO_URL: &str = "https://github.com/michaelyang12/knock.git";
const CARGO_TOML_URL: &str =
    "https://raw.githubusercontent.com/michaelyang12/knock/master/Cargo.toml";
const LOCAL_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> ExitCode {
    eprintln!("{}", "Checking for updates...".cyan());
    match remote_version() {
        Some(remote) if remote == LOCAL_VERSION => {
            eprintln!(
                "{}",
                format!("Already up to date (v{LOCAL_VERSION}).").green()
            );
            return ExitCode::SUCCESS;
        }
        Some(remote) => eprintln!(
            "{}",
            format!("Upgrading v{LOCAL_VERSION} -> v{remote}...").cyan()
        ),
        None => eprintln!(
            "{}",
            "Couldn't read the latest version; upgrading anyway.".yellow()
        ),
    }
    match Command::new("cargo")
        .args(["install", "--git", REPO_URL, "--locked", "--force"])
        .status()
    {
        Ok(s) if s.success() => {
            eprintln!("{}", "Upgrade complete.".green());
            ExitCode::SUCCESS
        }
        Ok(_) => {
            eprintln!("{} cargo install failed", "error:".red().bold());
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("{} could not run cargo: {e}", "error:".red().bold());
            ExitCode::FAILURE
        }
    }
}

fn remote_version() -> Option<String> {
    let out = Command::new("curl")
        .args(["-fsSL", "--max-time", "10", CARGO_TOML_URL])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    parse_version(&String::from_utf8_lossy(&out.stdout))
}

fn parse_version(cargo_toml: &str) -> Option<String> {
    cargo_toml
        .lines()
        .skip_while(|l| l.trim() != "[package]")
        .find_map(|l| {
            let (key, value) = l.split_once('=')?;
            (key.trim() == "version").then(|| value.trim().trim_matches('"').to_string())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_package_version_only() {
        let toml = "[package]\nname = \"knock\"\nversion = \"0.3.0\"\n\n[dependencies]\nclap = { version = \"4\" }\n";
        assert_eq!(parse_version(toml).as_deref(), Some("0.3.0"));
    }
}
