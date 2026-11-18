/// A local backstop for the model's own `dangerous` flag. It only needs to catch the obvious cases;
/// the model judges the rest.
const PATTERNS: &[&str] = &[
    "rm -rf",
    "rm -fr",
    "rm -r ",
    "sudo ",
    "doas ",
    "mkfs",
    "dd if=",
    "dd of=",
    "> /dev/",
    "chmod -r",
    "chown -r",
    "git push --force",
    "git push -f",
    "git reset --hard",
    "git clean -f",
    "git branch -d",
    "drop table",
    "drop database",
    "truncate table",
    "shutdown",
    "reboot",
    ":(){",
    "kill -9",
    "killall",
    "diskutil erase",
];

pub fn looks_dangerous(command: &str) -> bool {
    let lower = command.to_lowercase();
    PATTERNS.iter().any(|p| lower.contains(p))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_destructive_commands() {
        for cmd in [
            "rm -rf node_modules",
            "sudo apt install x",
            "git push --force origin main",
            "find . -name '*.log' | xargs rm -r ",
            "chmod -R 777 .",
        ] {
            assert!(looks_dangerous(cmd), "{cmd}");
        }
    }

    #[test]
    fn leaves_safe_commands_alone() {
        for cmd in [
            "ls -la",
            "find . -size +500M",
            "git status",
            "git log --format '%h %s'",
            "du -sh .",
        ] {
            assert!(!looks_dangerous(cmd), "{cmd}");
        }
    }
}
