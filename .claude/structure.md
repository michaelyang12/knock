# Knock - Code Structure

## Directory Layout

```
knock/
├── Cargo.toml          # Package manifest, dependencies, binary definition
├── Cargo.lock          # Locked dependency versions
├── README.md           # User documentation
├── .gitignore          # /target, .env, .idea
├── .claude/            # Claude Code documentation (this folder)
└── src/
    ├── main.rs         # Entry point, command routing, execution flow
    ├── args.rs         # CLI argument definitions (clap derive)
    ├── config.rs       # Configuration management, Provider enum
    ├── context.rs      # Shell/OS detection for AI context
    ├── client.rs       # AI API clients (OpenAI, Anthropic, Ollama)
    ├── cache.rs        # Query result caching (sled)
    ├── history.rs      # Command history tracking (sled)
    └── setup.rs        # Interactive configuration wizard
```

## Module Responsibilities

### `main.rs` - Core Orchestration
- `#[tokio::main]` async entry point
- CLI argument parsing via clap
- Command routing: setup, history, query, upgrade
- Request → cache check → API call → cache store → display
- Command execution with y/n confirmation
- Clipboard copy functionality
- Git-based self-upgrade

### `args.rs` - CLI Interface
- `Args` struct with clap derive
- `Commands` enum for subcommands (`Explain`)
- Flags: `--verbose` (explanation), `--alt` (alternatives), `--execute`, `--history`, `--config`, `--upgrade`
- Positional: query string
- Subcommand: `explain <command>` - explain what a shell command does

### `config.rs` - Configuration
- `Provider` enum: `OpenAI`, `Anthropic`, `Ollama`
- `Config` struct: provider, model overrides, custom URLs
- Load/save from `~/.knock/config.json`
- Default model mappings per provider

### `client.rs` - AI Abstraction
- `RequestClient` struct with unified interface
- `RequestMode` enum: `Standard`, `Verbose`, `Alt`, `Explain`
- Per-provider implementations:
  - `request_openai()` - async-openai library
  - `request_anthropic()` - raw HTTP POST
  - `request_ollama()` - local HTTP to Ollama server
- Two instruction sets: `INSTRUCTIONS` (command generation) and `EXPLAIN_INSTRUCTIONS` (command explanation)
- Temperature: 0.2, Max tokens: 256 (standard) or 512 (verbose/alt/explain)

### `context.rs` - Environment Detection
- `ShellContext` struct: OS, shell, cwd
- Auto-detects via env vars and `ps` fallback
- Formats as XML for AI prompt context

### `cache.rs` - Query Caching
- `Cache` struct wrapping sled DB
- Key: hash of (query + OS + shell + verbose)
- Stores serialized responses

### `history.rs` - History Management
- `History` struct wrapping sled DB
- `HistoryEntry`: query, command, timestamp
- Auto-prunes to 100 entries
- Search and recent retrieval

### `setup.rs` - Interactive Setup
- `run_setup()` wizard function
- Provider selection menu
- API key validation/warnings
- Ollama model fetching
- Saves to config.json
