use colored::Colorize;
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::process::{Command, ExitCode, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// A stderr spinner that only draws when stderr is a terminal, so it never pollutes pipes.
pub struct Spinner {
    stop: Arc<AtomicBool>,
    task: Option<tokio::task::JoinHandle<()>>,
}

impl Spinner {
    pub fn start(label: &str) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        if !std::io::stderr().is_terminal() {
            return Self { stop, task: None };
        }
        let flag = stop.clone();
        let label = label.to_string();
        let task = tokio::spawn(async move {
            const FRAMES: [char; 10] = ['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];
            let mut i = 0;
            while !flag.load(Ordering::Relaxed) {
                eprint!(
                    "\r{} {}",
                    FRAMES[i % FRAMES.len()].to_string().cyan(),
                    label.dimmed()
                );
                let _ = std::io::stderr().flush();
                i += 1;
                tokio::time::sleep(Duration::from_millis(80)).await;
            }
            eprint!("\r\x1b[2K");
            let _ = std::io::stderr().flush();
        });
        Self {
            stop,
            task: Some(task),
        }
    }

    pub async fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}

/// Copies to the system clipboard with whatever tool exists. Returns false if none worked.
pub fn copy(text: &str) -> bool {
    let candidates: &[(&str, &[&str])] = if cfg!(target_os = "macos") {
        &[("pbcopy", &[])]
    } else if cfg!(windows) {
        &[("clip", &[])]
    } else {
        &[
            ("wl-copy", &[]),
            ("xclip", &["-selection", "clipboard"]),
            ("xsel", &["--clipboard", "--input"]),
        ]
    };
    candidates
        .iter()
        .any(|(cmd, args)| pipe_to(cmd, args, text))
}

fn pipe_to(cmd: &str, args: &[&str], text: &str) -> bool {
    let Ok(mut child) = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let wrote = child
        .stdin
        .take()
        .is_some_and(|mut stdin| stdin.write_all(text.as_bytes()).is_ok());
    child.wait().is_ok_and(|s| s.success()) && wrote
}

/// Asks a yes/no question on the terminal, even when stdin is a pipe.
pub fn confirm(prompt: &str) -> bool {
    eprint!("{prompt}");
    let _ = std::io::stderr().flush();
    let mut line = String::new();
    let read = if std::io::stdin().is_terminal() {
        std::io::stdin().lock().read_line(&mut line).is_ok()
    } else {
        std::fs::File::open("/dev/tty")
            .and_then(|tty| BufReader::new(tty).read_line(&mut line))
            .is_ok()
    };
    read && matches!(line.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Runs the command in the user's shell and passes its exit status through.
pub fn execute(command: &str) -> ExitCode {
    let shell = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    match Command::new(&shell).arg("-c").arg(command).status() {
        Ok(status) => match status.code() {
            Some(0) => ExitCode::SUCCESS,
            Some(code) => {
                eprintln!("{}", format!("exited with code {code}").red());
                ExitCode::from(u8::try_from(code).unwrap_or(1))
            }
            None => ExitCode::FAILURE,
        },
        Err(e) => {
            eprintln!("{} could not run {shell}: {e}", "error:".red().bold());
            ExitCode::FAILURE
        }
    }
}

pub fn warn_dangerous() {
    eprintln!(
        "{} {}",
        "!".red().bold(),
        "This command may change or delete data. Review it before running.".red()
    );
}
