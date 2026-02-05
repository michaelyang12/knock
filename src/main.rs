mod args;
mod cache;
mod context;
mod history;

use std::io::{self, BufRead, Write};
use std::process::{Command, Stdio};

use crate::args::{Args, Commands};
use crate::cache::Cache;
use crate::context::ShellContext;
use crate::history::History;
use clap::Parser;
use colored::*;
use conduit::{Config, RequestOptions};

const INSTRUCTIONS: &str = r#"
<system_instructions>
  <role>
    You are a command-line translation assistant. Convert natural language into accurate, executable CLI commands.
  </role>

  <output_format>
    Return ONLY the command(s) needed. No explanations, no markdown, no preamble unless a special mode is enabled.
  </output_format>

  <command_chaining>
    Use appropriate operators: && (sequential), || (fallback), ; (independent), | (pipe)
  </command_chaining>

  <modes>
    <mode name="standard" default="true">
      Return the most direct, idiomatic command. Prioritize single-line solutions, common tools, and safe defaults.
    </mode>

    <mode name="verbose">
      When [verbose] flag is present, return the command on the first line, then a blank line, then a brief explanation (2-3 sentences) of what the command does and why any flags/options are used.
    </mode>

    <mode name="alt">
      When [alt] flag is present, return:
      PRIMARY: main command
      ALTERNATIVES: 2-3 alternatives with brief explanations
      OPTIONS: relevant flags that modify behavior
    </mode>
  </modes>

  <safety>
    For destructive operations (rm, format, drop), include confirmation flags unless "force" is in the request.
    For privilege escalation, prefix with sudo on Unix/Linux.
  </safety>

  <constraints>
    Never include explanatory text in standard mode.
    Don't ask clarifying questions; make reasonable assumptions.
    Use the provided OS, shell, and cwd context to generate accurate commands.
  </constraints>
</system_instructions>
"#;

const EXPLAIN_INSTRUCTIONS: &str = r#"
<system_instructions>
  <role>
    You are a command-line expert. Explain what shell commands do in clear, concise terms.
  </role>

  <output_format>
    Provide a clear explanation of the command:
    1. Start with a one-sentence summary of what the command does
    2. Break down each part: the base command, flags/options, and arguments
    3. Mention any important side effects or gotchas
    Keep it concise but thorough.
  </output_format>

  <invalid_commands>
    If the input is not a valid or recognizable shell command, respond with exactly:
    Invalid command.
    Do not elaborate or explain why it's invalid.
  </invalid_commands>

  <constraints>
    Don't suggest alternatives or improvements unless the command is dangerous.
    Focus on explaining what the given command does, not what else could be done.
    Use the provided OS and shell context to give accurate, platform-specific details.
  </constraints>
</system_instructions>
"#;

/// The type of request being made
#[derive(Clone, Copy)]
enum RequestMode {
    Standard,
    Verbose,
    Alt,
    Explain,
}

fn gen_prompt(context: &ShellContext, input: &str, mode: RequestMode) -> String {
    let mode_tag = match mode {
        RequestMode::Standard => "",
        RequestMode::Verbose => " [verbose]",
        RequestMode::Alt => " [alt]",
        RequestMode::Explain => "",
    };
    format!(
        "{}\n\n<request>{}{}</request>",
        context.as_prompt_context(),
        input,
        mode_tag
    )
}

fn get_instructions(mode: RequestMode) -> &'static str {
    match mode {
        RequestMode::Explain => EXPLAIN_INSTRUCTIONS,
        _ => INSTRUCTIONS,
    }
}

fn get_max_tokens(mode: RequestMode) -> u32 {
    match mode {
        RequestMode::Standard => 256,
        RequestMode::Verbose | RequestMode::Alt | RequestMode::Explain => 512,
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    if args.upgrade {
        upgrade();
        return;
    }

    if args.config {
        conduit::setup::run_setup("knock", None);
        return;
    }

    if args.history {
        show_history(&args.input);
        return;
    }

    // Handle explain subcommand
    if let Some(Commands::Explain { command }) = &args.command {
        explain_command(command, &args).await;
        return;
    }

    if args.input.is_empty() {
        eprintln!("{}", "Error: Please provide a query".red());
        std::process::exit(1);
    }

    // Determine request mode
    let mode = if args.alt {
        RequestMode::Alt
    } else if args.verbose {
        RequestMode::Verbose
    } else {
        RequestMode::Standard
    };

    let mode_str = match mode {
        RequestMode::Standard => "standard",
        RequestMode::Verbose => "verbose",
        RequestMode::Alt => "alt",
        RequestMode::Explain => "explain",
    };

    let context = ShellContext::detect();
    let config = Config::load("knock");
    let cache_key = Cache::generate_key(&args.input, &context.os, &context.shell, mode_str);
    let cache = Cache::load();

    let res = if let Some(cached) = cache.get(&cache_key) {
        cached
    } else {
        let prompt = gen_prompt(&context, &args.input, mode);
        let options = RequestOptions {
            max_tokens: get_max_tokens(mode),
            temperature: 0.2,
            model: None,
        };

        let response = conduit::request(&config, get_instructions(mode), &prompt, options)
            .await
            .expect("Error getting response");

        cache.insert(cache_key, response.clone());
        response
    };

    // Save to history (only for standard/verbose, not alt)
    if !args.alt {
        let history = History::load();
        // For verbose mode, extract just the command (first line) for history
        let cmd_for_history = if args.verbose {
            res.lines().next().unwrap_or(&res).to_string()
        } else {
            res.clone()
        };
        history.add(args.input.clone(), cmd_for_history);
    }

    // Display result
    if args.verbose {
        // Verbose: command on first line, explanation below
        let mut lines = res.lines();
        if let Some(cmd) = lines.next() {
            println!("{}", cmd.bright_green());
            // Copy just the command to clipboard
            copy_to_clipboard(cmd).expect("Error copying to clipboard");
            // Print remaining lines (explanation) in dimmed style
            let explanation: String = lines.collect::<Vec<_>>().join("\n");
            if !explanation.trim().is_empty() {
                println!("{}", explanation.trim().dimmed());
            }
        }
    } else {
        println!("{}", &res.bright_green());
        if !args.alt {
            copy_to_clipboard(&res).expect("Error copying to clipboard");
        }
    }

    if args.execute && !args.alt {
        let cmd = if args.verbose {
            res.lines().next().unwrap_or(&res)
        } else {
            &res
        };
        print!("{}", "Execute? [y/N] ".yellow());
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().lock().read_line(&mut input).unwrap();

        if input.trim().eq_ignore_ascii_case("y") {
            println!("{}", "---".dimmed());
            execute_command(cmd);
        }
    }
}

async fn explain_command(command: &str, _args: &Args) {
    let context = ShellContext::detect();
    let config = Config::load("knock");
    let cache_key = Cache::generate_key(command, &context.os, &context.shell, "explain");
    let cache = Cache::load();

    let res = if let Some(cached) = cache.get(&cache_key) {
        cached
    } else {
        let prompt = gen_prompt(&context, command, RequestMode::Explain);
        let options = RequestOptions {
            max_tokens: get_max_tokens(RequestMode::Explain),
            temperature: 0.2,
            model: None,
        };

        let response = conduit::request(&config, get_instructions(RequestMode::Explain), &prompt, options)
            .await
            .expect("Error getting response");

        cache.insert(cache_key, response.clone());
        response
    };

    println!("{}", command.bright_green());
    println!();
    println!("{}", res);
}

fn show_history(filter: &str) {
    let history = History::load();
    let entries = if filter.is_empty() {
        history.recent(20)
    } else {
        history.search(filter)
    };

    if entries.is_empty() {
        println!("{}", "No history found.".dimmed());
        return;
    }

    for entry in entries.iter() {
        println!("{}", entry.query.dimmed());
        println!("  {}", entry.command.bright_green());
    }
}

fn execute_command(cmd: &str) {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    let status = Command::new(&shell)
        .arg("-c")
        .arg(cmd)
        .status();

    match status {
        Ok(s) if !s.success() => {
            if let Some(code) = s.code() {
                eprintln!("{}", format!("Command exited with code {}", code).red());
            }
        }
        Err(e) => eprintln!("{}", format!("Failed to execute: {}", e).red()),
        _ => {}
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

const REPO_URL: &str = "https://github.com/michaelyang12/knock.git";
const CARGO_TOML_URL: &str = "https://raw.githubusercontent.com/michaelyang12/knock/master/Cargo.toml";
const LOCAL_VERSION: &str = env!("CARGO_PKG_VERSION");

fn get_remote_version() -> Option<String> {
    let output = Command::new("curl")
        .args(["-sL", CARGO_TOML_URL])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let content = String::from_utf8(output.stdout).ok()?;
    for line in content.lines() {
        if line.starts_with("version") {
            let version = line
                .split('=')
                .nth(1)?
                .trim()
                .trim_matches('"');
            return Some(version.to_string());
        }
    }
    None
}

fn upgrade() {
    println!("{}", "Checking for updates...".cyan());

    match get_remote_version() {
        Some(remote_version) if remote_version == LOCAL_VERSION => {
            println!(
                "{}",
                format!("Already up to date (v{}).", LOCAL_VERSION).green()
            );
            return;
        }
        Some(remote_version) => {
            println!(
                "{}",
                format!("Upgrading from v{} to v{}...", LOCAL_VERSION, remote_version).cyan()
            );
        }
        None => {
            println!("{}", "Could not check remote version, upgrading anyway...".yellow());
        }
    }

    let status = Command::new("cargo")
        .args(["install", "--git", REPO_URL, "--locked", "--force"])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("{}", "Upgrade complete!".green());
        }
        Ok(_) => {
            eprintln!("{}", "Upgrade failed".red());
        }
        Err(e) => {
            eprintln!("{}", format!("Failed to run cargo: {}", e).red());
        }
    }
}
