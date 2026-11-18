#[cfg(not(unix))]
use std::io::IsTerminal;
use std::io::Read;
use std::process::Command;

/// Piped input beyond this is dropped from the front; errors usually sit at the end.
const MAX_STDIN_CHARS: usize = 12_000;

pub struct ShellContext {
    pub os: String,
    pub shell: String,
    pub cwd: String,
}

impl ShellContext {
    pub fn detect() -> Self {
        Self {
            os: detect_os().to_string(),
            shell: detect_shell(),
            cwd: std::env::current_dir()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "unknown".to_string()),
        }
    }
}

fn detect_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "macOS",
        "linux" => "Linux",
        "windows" => "Windows",
        "freebsd" => "FreeBSD",
        other => other,
    }
}

fn detect_shell() -> String {
    if let Some(name) = std::env::var("SHELL").ok().as_deref().and_then(basename) {
        return name.to_string();
    }
    if cfg!(windows) {
        return if std::env::var_os("PSModulePath").is_some() {
            "powershell".into()
        } else {
            "cmd".into()
        };
    }
    parent_process_name().unwrap_or_else(|| "sh".into())
}

fn parent_process_name() -> Option<String> {
    let ppid = run(
        "ps",
        &["-p", &std::process::id().to_string(), "-o", "ppid="],
    )?;
    let comm = run("ps", &["-p", ppid.trim(), "-o", "comm="])?;
    basename(comm.trim().trim_start_matches('-')).map(str::to_string)
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn basename(path: &str) -> Option<&str> {
    path.rsplit('/').next().filter(|s| !s.is_empty())
}

/// Reads piped stdin, if any. Returns None when stdin is a terminal or empty.
pub fn piped_input() -> Option<String> {
    let mut stdin = std::io::stdin();
    if !stdin_has_input() {
        return None;
    }
    let mut buf = Vec::new();
    stdin.read_to_end(&mut buf).ok()?;
    let text = String::from_utf8_lossy(&buf);
    let text = text.trim();
    (!text.is_empty()).then(|| keep_tail(text, MAX_STDIN_CHARS))
}

/// Only pipes and redirected files count. Sockets and other inherited handles can stay open
/// without ever sending data or EOF (IDE task runners, `ssh -T`), and reading them would hang.
#[cfg(unix)]
fn stdin_has_input() -> bool {
    use std::os::fd::AsFd;
    use std::os::unix::fs::FileTypeExt;
    let Ok(fd) = std::io::stdin().as_fd().try_clone_to_owned() else {
        return false;
    };
    std::fs::File::from(fd)
        .metadata()
        .map(|m| m.file_type().is_fifo() || m.file_type().is_file())
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn stdin_has_input() -> bool {
    !std::io::stdin().is_terminal()
}

fn keep_tail(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_string();
    }
    let tail: String = text.chars().skip(count - max).collect();
    format!("[...earlier output truncated]\n{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn long_input_keeps_the_end() {
        let text = format!("{}ERROR at the end", "x".repeat(50));
        let kept = keep_tail(&text, 16);
        assert!(kept.ends_with("ERROR at the end"));
        assert!(kept.starts_with("[...earlier output truncated]"));
        assert_eq!(keep_tail("short", 16), "short");
    }

    #[test]
    fn basename_handles_login_shell_paths() {
        assert_eq!(basename("/bin/zsh"), Some("zsh"));
        assert_eq!(basename("fish"), Some("fish"));
        assert_eq!(basename("/bin/"), None);
    }
}
