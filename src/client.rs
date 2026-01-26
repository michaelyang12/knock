use crate::args::Args;
use crate::config::{Config, Provider};
use crate::context::ShellContext;
use async_openai::{
    Client, config::OpenAIConfig, error::OpenAIError, types::responses::CreateResponseArgs,
};
use serde::{Deserialize, Serialize};

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

pub struct RequestClient {
    args: Args,
    context: ShellContext,
    config: Config,
}

/// The type of request being made
#[derive(Clone, Copy)]
pub enum RequestMode {
    Standard,
    Verbose,
    Alt,
    Explain,
}

impl RequestClient {
    pub fn new(args: Args, context: ShellContext, config: Config) -> Self {
        Self { args, context, config }
    }

    fn gen_prompt(&self, mode: RequestMode) -> String {
        let mode_tag = match mode {
            RequestMode::Standard => "",
            RequestMode::Verbose => " [verbose]",
            RequestMode::Alt => " [alt]",
            RequestMode::Explain => "",
        };
        format!(
            "{}\n\n<request>{}{}</request>",
            self.context.as_prompt_context(),
            self.args.input,
            mode_tag
        )
    }

    fn get_instructions(mode: RequestMode) -> &'static str {
        match mode {
            RequestMode::Explain => EXPLAIN_INSTRUCTIONS,
            _ => INSTRUCTIONS,
        }
    }

    fn get_max_tokens(&self, mode: RequestMode) -> u32 {
        match mode {
            RequestMode::Standard => 256,
            RequestMode::Verbose | RequestMode::Alt | RequestMode::Explain => 512,
        }
    }

    pub async fn make_request(&self, mode: RequestMode) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        match self.config.provider {
            Provider::OpenAI => self.request_openai(mode).await,
            Provider::Anthropic => self.request_anthropic(mode).await,
            Provider::Ollama => self.request_ollama(mode).await,
        }
    }

    async fn request_openai(&self, mode: RequestMode) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let client: Client<OpenAIConfig> = Client::new();
        let prompt = self.gen_prompt(mode);
        let request = CreateResponseArgs::default()
            .model(self.config.openai_model())
            .instructions(Self::get_instructions(mode))
            .input(prompt)
            .temperature(0.2)
            .max_output_tokens(self.get_max_tokens(mode))
            .build()?;

        let response = client.responses().create(request).await?;

        if let Some(text) = response.output_text() {
            Ok(text.clone())
        } else {
            Err(OpenAIError::InvalidArgument("Empty response".to_string()).into())
        }
    }

    async fn request_anthropic(&self, mode: RequestMode) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY not set")?;

        let prompt = self.gen_prompt(mode);

        let request_body = AnthropicRequest {
            model: self.config.anthropic_model().to_string(),
            max_tokens: self.get_max_tokens(mode),
            system: Self::get_instructions(mode).to_string(),
            messages: vec![AnthropicMessage {
                role: "user".to_string(),
                content: prompt,
            }],
        };

        let client = reqwest::Client::new();
        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Anthropic API error: {}", error_text).into());
        }

        let response_body: AnthropicResponse = response.json().await?;

        response_body
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| "Empty response from Anthropic".into())
    }

    async fn request_ollama(&self, mode: RequestMode) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let prompt = self.gen_prompt(mode);
        let url = format!("{}/api/chat", self.config.ollama_url());

        let request_body = OllamaRequest {
            model: self.config.ollama_model().to_string(),
            messages: vec![
                OllamaMessage {
                    role: "system".to_string(),
                    content: Self::get_instructions(mode).to_string(),
                },
                OllamaMessage {
                    role: "user".to_string(),
                    content: prompt,
                },
            ],
            stream: false,
            options: OllamaOptions {
                temperature: 0.2,
                num_predict: self.get_max_tokens(mode),
            },
        };

        let client = reqwest::Client::new();
        let response = client
            .post(&url)
            .json(&request_body)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Ollama error: {}. Is Ollama running?", error_text).into());
        }

        let response_body: OllamaResponse = response.json().await?;
        Ok(response_body.message.content)
    }
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}
