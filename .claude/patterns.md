# Knock - Code Patterns & Conventions

## Error Handling

Uses `anyhow` for error propagation with `?` operator throughout. Functions return `anyhow::Result<T>` for consistent error handling.

```rust
pub fn load() -> anyhow::Result<Self> {
    let path = Self::path()?;
    // ...
}
```

## Configuration Pattern

Config uses a builder-like pattern with defaults:

```rust
impl Default for Config {
    fn default() -> Self {
        Self {
            provider: Provider::OpenAI,
            openai_model: None,      // Falls back to default
            anthropic_model: None,
            ollama_model: None,
            // ...
        }
    }
}
```

## Provider Abstraction

The `Provider` enum with match statements for provider-specific logic:

```rust
match config.provider {
    Provider::OpenAI => client.request_openai(&query, verbose).await,
    Provider::Anthropic => client.request_anthropic(&query, verbose).await,
    Provider::Ollama => client.request_ollama(&query, verbose).await,
}
```

## Sled Database Usage

Both cache and history use sled's embedded key-value store:

```rust
// Open/create DB
let db = sled::open(&path)?;

// Insert (key and value must be bytes)
db.insert(key.as_bytes(), value.as_bytes())?;

// Get
if let Some(bytes) = db.get(key.as_bytes())? {
    let value = String::from_utf8(bytes.to_vec())?;
}

// Iterate
for result in db.iter() {
    let (key, value) = result?;
}
```

## CLI Argument Style

Uses clap derive macros with short/long flags:

```rust
#[derive(Parser, Debug)]
#[command(name = "kn")]
pub struct Args {
    #[arg(short, long)]
    pub verbose: bool,

    #[arg(short, long)]
    pub execute: bool,

    pub query: Option<String>,
}
```

## Terminal Output

Uses `colored` crate for styled output:

```rust
println!("{}", "Command:".green().bold());
println!("{}", command.cyan());
```

## Async Pattern

Main uses tokio runtime, but some operations (like reqwest) use blocking:

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // async operations with .await
}

// In client.rs, some requests use reqwest::blocking
let response = reqwest::blocking::Client::new()
    .post(url)
    .json(&body)
    .send()?;
```

## Path Conventions

All user data under `~/.knock/`:

```rust
fn path() -> anyhow::Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("No home directory"))?;
    Ok(home.join(".knock").join("config.json"))
}
```
