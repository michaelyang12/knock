# Knock - Project Overview

## What is Knock?

**Knock** is a CLI tool that translates natural language into shell commands using AI. Users describe what they want in plain English, and the tool generates the corresponding command.

```bash
kn "find all large files over 500MB"
→ find . -type f -size +500M
```

## Key Features

- Natural language → shell command translation
- **Explain mode**: `kn explain "ls -la"` explains what a command does
- Multi-provider AI support (OpenAI, Anthropic, Ollama)
- Query result caching (sled embedded DB)
- Command history tracking
- Verbose mode (`-v`): command + explanation of what it does
- Alt mode (`-a`): show alternative commands and options
- Command execution with confirmation (`-x`)
- Interactive configuration setup (`kn --config`)
- Self-upgrade capability (`kn --upgrade`)

## Tech Stack

- **Language:** Rust (2024 edition)
- **CLI Framework:** clap 4.x (derive macros)
- **HTTP:** reqwest (blocking)
- **Async:** tokio
- **Database:** sled (embedded key-value store)
- **Serialization:** serde/serde_json

## Binary

- **Name:** `kn`
- **Entry:** `src/main.rs`
- **Version:** 0.2.1

## Data Storage

All user data stored in `~/.knock/`:
- `config.json` - Provider/model configuration
- `cache/` - Sled DB for query caching
- `history/` - Sled DB for command history

## Supported Providers

| Provider  | Env Variable       | Default Model               |
|-----------|--------------------|-----------------------------|
| OpenAI    | OPENAI_API_KEY     | gpt-4o-mini                 |
| Anthropic | ANTHROPIC_API_KEY  | claude-sonnet-4-20250514            |
| Ollama    | None (local)       | llama3.2                    |
