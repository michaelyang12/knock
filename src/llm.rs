use crate::context::ShellContext;
use crate::danger;
use conduit::{Conduit, Config, Instructions, Provider, Reasoning, Request};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

pub const APP: &str = "knock";
const MAX_TOKENS: u32 = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Standard,
    Verbose,
    Alt,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Standard => "standard",
            Mode::Verbose => "verbose",
            Mode::Alt => "alt",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Suggestion {
    pub command: String,
    #[serde(default)]
    pub dangerous: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub explanation: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alternatives: Vec<Alternative>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Alternative {
    pub command: String,
    pub note: String,
}

impl Suggestion {
    /// Cleans up wrappers models add anyway and applies the local danger check on top of the model's.
    fn normalize(mut self) -> Self {
        self.command = clean_command(&self.command);
        for alt in &mut self.alternatives {
            alt.command = clean_command(&alt.command);
        }
        self.dangerous |= danger::looks_dangerous(&self.command);
        self
    }
}

fn clean_command(command: &str) -> String {
    let trimmed = command.trim();
    let unfenced = trimmed
        .strip_prefix("```")
        .map(|rest| rest.split_once('\n').map_or(rest, |(_, body)| body))
        .and_then(|body| body.trim_end().strip_suffix("```"))
        .unwrap_or(trimmed);
    unfenced.trim().trim_matches('`').trim().to_string()
}

/// Loads the configured LLM, with `--provider` / `--model` taking precedence over config and env.
pub fn connect(provider: Option<&str>, model: Option<&str>) -> conduit::Result<Conduit> {
    let config = Config::load(APP)?;
    let mut llm = match provider {
        Some(name) => {
            let conduit = Conduit::new(Provider::resolve(name, config.providers.get(name))?);
            match config.timeout_secs {
                Some(secs) => conduit.with_timeout(Duration::from_secs(secs)),
                None => conduit,
            }
        }
        None => Conduit::from_config(&config)?,
    };
    if let Some(model) = model {
        llm = llm.with_model(model);
    }
    Ok(llm)
}

fn base_request<'a>(
    llm: &'a Conduit,
    ctx: &ShellContext,
    input: &str,
    piped: Option<&str>,
) -> Request<'a> {
    let mut req = llm
        .prompt(input)
        .context("os", &ctx.os)
        .context("shell", &ctx.shell)
        .context("cwd", &ctx.cwd)
        .max_tokens(MAX_TOKENS)
        .reasoning(Reasoning::Off);
    if let Some(piped) = piped {
        req = req.context("piped_input", piped);
    }
    let provider = llm.provider();
    if provider.is_local() && !provider.protocol.is_cli() {
        req = req.temperature(0.2);
    }
    req
}

pub async fn suggest(
    llm: &Conduit,
    ctx: &ShellContext,
    query: &str,
    piped: Option<&str>,
    mode: Mode,
) -> conduit::Result<Suggestion> {
    let suggestion: Suggestion = base_request(llm, ctx, query, piped)
        .system(command_instructions(mode))
        .schema(schema(mode))
        .json()
        .await?;
    Ok(suggestion.normalize())
}

pub async fn explain(llm: &Conduit, ctx: &ShellContext, command: &str) -> conduit::Result<String> {
    base_request(llm, ctx, command, None)
        .system(explain_instructions())
        .text()
        .await
}

fn command_instructions(mode: Mode) -> Instructions {
    let builder = Instructions::builder()
        .role("You translate natural-language requests into shell commands for the user's OS and shell.")
        .rule("Return the most direct, idiomatic command. Prefer a single line and tools that ship with the OS.")
        .rule("Chain steps with && (sequential), || (fallback), ; (independent) or | (pipe) when one command is not enough.")
        .rule("Use the context: the os and shell decide tools and flags (macOS ships BSD utilities, not GNU), and cwd is where the command runs.")
        .rule("Keep confirmation flags on destructive operations unless the request asks to force.")
        .rule("Never ask clarifying questions; make reasonable assumptions.")
        .rule("Set dangerous to true when the command deletes or overwrites data, changes permissions or system settings, needs sudo, rewrites git history, or affects remote systems.")
        .rule("If piped_input is in the context, it is output the user is asking about (often an error); return the command that addresses the request in light of it.");
    let builder = match mode {
        Mode::Standard => builder
            .format("JSON with `command` and `dangerous`.")
            .example(
                "find files over 500MB here",
                r#"{"command": "find . -type f -size +500M", "dangerous": false}"#,
            )
            .example(
                "undo my last commit but keep the changes",
                r#"{"command": "git reset --soft HEAD~1", "dangerous": true}"#,
            ),
        Mode::Verbose => builder.format(
            "JSON with `command`, `dangerous`, and `explanation`: two or three plain sentences on what the command does and why each flag is there. No markdown.",
        ),
        Mode::Alt => builder.format(
            "JSON with `command` (the best option), `dangerous`, and `alternatives`: two or three other commands that achieve the same goal, each with a short `note` on when to prefer it.",
        ),
    };
    builder.build()
}

fn schema(mode: Mode) -> Value {
    let mut properties = json!({
        "command": { "type": "string", "description": "The shell command, ready to paste" },
        "dangerous": { "type": "boolean" },
    });
    let mut required = vec!["command", "dangerous"];
    match mode {
        Mode::Standard => {}
        Mode::Verbose => {
            properties["explanation"] = json!({ "type": "string" });
            required.push("explanation");
        }
        Mode::Alt => {
            properties["alternatives"] = json!({
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "command": { "type": "string" },
                        "note": { "type": "string" }
                    },
                    "required": ["command", "note"],
                    "additionalProperties": false
                }
            });
            required.push("alternatives");
        }
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
}

fn explain_instructions() -> Instructions {
    Instructions::builder()
        .role("You are a command-line expert who explains what shell commands do.")
        .format(
            "Start with a one-sentence summary. Then explain each part (the program, each flag, each argument) on its own line as `part: meaning`. End with any side effects or gotchas, if there are any. Plain text, no markdown headings or code fences.",
        )
        .rule("If the input is not a recognizable shell command, reply with exactly: Invalid command.")
        .rule("Don't suggest alternatives unless the command is dangerous.")
        .rule("Use the os and shell in the context for platform-specific details.")
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_fences_and_backticks() {
        assert_eq!(clean_command("`ls -la`"), "ls -la");
        assert_eq!(clean_command("```bash\nls -la\n```"), "ls -la");
        assert_eq!(clean_command("  du -sh .  "), "du -sh .");
    }

    #[test]
    fn local_check_backs_up_the_model() {
        let s = Suggestion {
            command: "rm -rf build".into(),
            dangerous: false,
            explanation: None,
            alternatives: vec![],
        };
        assert!(s.normalize().dangerous);
    }

    #[test]
    fn schemas_require_mode_fields() {
        assert_eq!(
            schema(Mode::Standard)["required"],
            json!(["command", "dangerous"])
        );
        assert_eq!(
            schema(Mode::Verbose)["required"],
            json!(["command", "dangerous", "explanation"])
        );
        assert!(schema(Mode::Alt)["properties"]["alternatives"].is_object());
    }

    #[test]
    fn suggestions_tolerate_missing_optional_fields() {
        let s: Suggestion = serde_json::from_str(r#"{"command": "ls"}"#).unwrap();
        assert!(!s.dangerous);
        assert!(s.alternatives.is_empty());
    }
}
