//! `kn --init <shell>` output. The wrapper only intercepts plain queries (no leading flag and no
//! `explain`), runs `kn --raw`, and puts the result on the prompt line for editing. Everything
//! else passes straight through to the binary.

use crate::args::Shell;

const ZSH: &str = r#"kn() {
  case "$1" in
    ""|-*|explain) command kn "$@"; return ;;
  esac
  local cmd
  cmd="$(command kn --raw "$@")" || return
  [ -n "$cmd" ] && print -z -- "$cmd"
}
"#;

const BASH: &str = r#"kn() {
  case "$1" in
    ""|-*|explain) command kn "$@"; return ;;
  esac
  local cmd
  cmd="$(command kn --raw "$@")" || return
  [ -n "$cmd" ] || return
  if [ "${BASH_VERSINFO[0]}" -ge 4 ]; then
    read -e -r -i "$cmd" -p "$ " cmd || return
    history -s -- "$cmd"
    eval -- "$cmd"
  else
    printf '%s\n' "$cmd"
  fi
}
"#;

const FISH: &str = r#"function kn --description 'knock: describe a command, edit it on the prompt line'
    if test (count $argv) -eq 0; or string match -q -- '-*' $argv[1]; or test "$argv[1]" = explain
        command kn $argv
        return
    end
    set -l cmd (command kn --raw $argv | string collect); or return
    test -n "$cmd"; or return
    read --command "$cmd" --prompt-str '$ ' cmd; or return
    eval $cmd
end
"#;

pub fn init_script(shell: Shell) -> &'static str {
    match shell {
        Shell::Zsh => ZSH,
        Shell::Bash => BASH,
        Shell::Fish => FISH,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shell_wraps_with_raw_and_passes_flags_through() {
        for shell in [Shell::Zsh, Shell::Bash, Shell::Fish] {
            let script = init_script(shell);
            assert!(script.contains("command kn --raw"), "{shell:?}");
            assert!(script.contains("explain"), "{shell:?}");
        }
    }

    #[test]
    fn scripts_parse_in_their_shell() {
        for (shell, bin, flag) in [(Shell::Zsh, "zsh", "-n"), (Shell::Bash, "bash", "-n")] {
            let Ok(out) = std::process::Command::new(bin)
                .args([flag, "-c", init_script(shell)])
                .output()
            else {
                continue;
            };
            assert!(
                out.status.success(),
                "{bin}: {}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
    }
}
