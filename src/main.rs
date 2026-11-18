mod args;
mod context;
mod danger;
mod llm;
mod shell;
mod store;
mod ui;
mod upgrade;

use crate::args::{Args, Command};
use crate::context::ShellContext;
use crate::llm::{APP, Mode, Suggestion};
use crate::store::{Cache, History};
use crate::ui::Spinner;
use clap::Parser;
use colored::Colorize;
use conduit::Conduit;
use std::io::IsTerminal;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    let args = Args::parse();
    if !std::io::stdout().is_terminal() && !std::io::stderr().is_terminal() {
        colored::control::set_override(false);
    }

    if let Some(shell) = args.init {
        print!("{}", shell::init_script(shell));
        return ExitCode::SUCCESS;
    }
    if args.upgrade {
        return upgrade::run();
    }
    if args.config {
        return match conduit::setup::run(APP).await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => report(&e),
        };
    }
    if args.history {
        show_history(&args.query());
        return ExitCode::SUCCESS;
    }
    if let Some(Command::Explain { command }) = &args.command {
        return explain(&args, &command.join(" ")).await;
    }

    let query = args.query();
    if query.is_empty() {
        eprintln!("usage: kn <what you want to do>    (kn --help for more)");
        return ExitCode::from(2);
    }
    suggest(&args, &query).await
}

async fn suggest(args: &Args, query: &str) -> ExitCode {
    let mode = if args.alt {
        Mode::Alt
    } else if args.verbose {
        Mode::Verbose
    } else {
        Mode::Standard
    };
    let piped = context::piped_input();
    let ctx = ShellContext::detect();
    let llm = match llm::connect(args.provider.as_deref(), args.model.as_deref()) {
        Ok(llm) => llm,
        Err(e) => return report(&e),
    };

    let dir = store::data_dir();
    let mut cache = Cache::open(&dir);
    let key = cache_key(&llm, mode.as_str(), &ctx, query);
    let use_cache = !args.fresh && piped.is_none();
    let cached = use_cache
        .then(|| cache.get(&key))
        .flatten()
        .and_then(|v| serde_json::from_str::<Suggestion>(v).ok());

    let suggestion = match cached {
        Some(s) => s,
        None => {
            let spinner = Spinner::start("thinking");
            let result = llm::suggest(&llm, &ctx, query, piped.as_deref(), mode).await;
            spinner.stop().await;
            match result {
                Ok(s) => {
                    if use_cache && let Ok(json) = serde_json::to_string(&s) {
                        cache.put(key, json);
                    }
                    s
                }
                Err(e) => return report(&e),
            }
        }
    };
    History::open(&dir).add(query, &suggestion.command);

    if args.raw || !std::io::stdout().is_terminal() {
        if suggestion.dangerous {
            ui::warn_dangerous();
        }
        println!("{}", suggestion.command);
        return ExitCode::SUCCESS;
    }

    print_suggestion(&suggestion);
    if !args.no_copy && ui::copy(&suggestion.command) && !args.execute {
        eprintln!("{}", "copied to clipboard".dimmed());
    }
    if args.execute {
        let question = if suggestion.dangerous {
            format!("{} ", "Run it anyway? [y/N]".red().bold())
        } else {
            format!("{} ", "Run it? [y/N]".yellow())
        };
        if ui::confirm(&question) {
            eprintln!("{}", "---".dimmed());
            return ui::execute(&suggestion.command);
        }
    }
    ExitCode::SUCCESS
}

fn print_suggestion(s: &Suggestion) {
    println!("{}", s.command.bright_green());
    if let Some(explanation) = s.explanation.as_deref().filter(|e| !e.trim().is_empty()) {
        println!("\n{}", explanation.trim().dimmed());
    }
    if !s.alternatives.is_empty() {
        println!("\n{}", "alternatives".dimmed());
        for (i, alt) in s.alternatives.iter().enumerate() {
            println!("  {}. {}", i + 1, alt.command.green());
            println!("     {}", alt.note.dimmed());
        }
    }
    if s.dangerous {
        ui::warn_dangerous();
    }
}

async fn explain(args: &Args, command: &str) -> ExitCode {
    let ctx = ShellContext::detect();
    let llm = match llm::connect(args.provider.as_deref(), args.model.as_deref()) {
        Ok(llm) => llm,
        Err(e) => return report(&e),
    };
    let mut cache = Cache::open(&store::data_dir());
    let key = cache_key(&llm, "explain", &ctx, command);
    let cached = (!args.fresh)
        .then(|| cache.get(&key).map(str::to_string))
        .flatten();

    let text = match cached {
        Some(text) => text,
        None => {
            let spinner = Spinner::start("reading");
            let result = llm::explain(&llm, &ctx, command).await;
            spinner.stop().await;
            match result {
                Ok(text) => {
                    cache.put(key, text.clone());
                    text
                }
                Err(e) => return report(&e),
            }
        }
    };
    println!("{}\n\n{}", command.bright_green(), text);
    ExitCode::SUCCESS
}

fn cache_key(llm: &Conduit, mode: &str, ctx: &ShellContext, input: &str) -> String {
    let provider = llm.provider();
    Cache::key(&[
        mode,
        &provider.name,
        provider.model.as_deref().unwrap_or_default(),
        &ctx.os,
        &ctx.shell,
        input,
    ])
}

fn show_history(term: &str) {
    let history = History::open(&store::data_dir());
    let entries = if term.is_empty() {
        history.recent(20)
    } else {
        history.search(term)
    };
    if entries.is_empty() {
        println!("{}", "No history found.".dimmed());
    }
    for entry in entries {
        println!("{}", entry.query.dimmed());
        println!("  {}", entry.command.bright_green());
    }
}

fn report(e: &conduit::Error) -> ExitCode {
    use conduit::Error;
    eprintln!("{} {e}", "error:".red().bold());
    let hint = match e {
        Error::MissingModel { .. }
        | Error::MissingApiKey { .. }
        | Error::UnknownProvider(_)
        | Error::Config { .. }
        | Error::CommandNotFound { .. }
        | Error::Unauthorized { .. }
        | Error::ModelNotFound { .. } => Some("run `kn --config` to choose a provider and model"),
        Error::Truncated { .. } | Error::Empty { .. } => {
            Some("try again, or pick another model with --model")
        }
        Error::Json { raw, .. } => {
            eprintln!("{}", raw.trim().dimmed());
            Some(
                "the model didn't return a usable command; try again or pick another model with --model",
            )
        }
        _ => None,
    };
    if let Some(hint) = hint {
        eprintln!("  {}", hint.dimmed());
    }
    ExitCode::FAILURE
}
