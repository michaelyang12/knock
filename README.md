# knock

A CLI command helper written in Rust. Describe a command you need, get the shell command back.

    kn "find all large files over 500MB"
    → find . -type f -size +500M

---

## Installation

Requires Rust and Cargo.

    cargo install --git https://github.com/michaelyang12/knock.git --locked

This installs the `kn` binary into ~/.cargo/bin

---

## Setup

Run the interactive setup to configure your provider and model:

    kn --config

Or manually set your API key in your shell profile (.bashrc, .zshrc, etc.):

```bash
# OpenAI (default)
export OPENAI_API_KEY="your_key_here"

# Anthropic
export ANTHROPIC_API_KEY="your_key_here"

# Ollama - no API key needed (runs locally)
```

---

## Usage

    kn "undo last git commit but keep changes"     # basic query
    kn -v "list docker containers"                 # verbose (alternatives + options)
    kn -x "delete node_modules"                    # execute with confirmation
    kn --history                                   # show recent history
    kn --history "git"                             # search history
    kn --config                                    # configure provider/model
    kn --upgrade                                   # upgrade to latest version

---

## Configuration

Run `kn --config` for interactive setup, or create `~/.knock/config.json` manually:

**Supported providers:**

| Provider | Env Variable | Default Model |
|----------|--------------|---------------|
| `openai` | `OPENAI_API_KEY` | `gpt-4o-mini` |
| `anthropic` | `ANTHROPIC_API_KEY` | `claude-sonnet-4-20250514` |
| `ollama` | None (local) | `llama3.2` |

**Example configs:**

```json
// Use Claude
{ "provider": "anthropic" }

// Use GPT-5.1
{ "openai_model": "gpt-5.1" }

// Use Ollama locally (free, private)
{ "provider": "ollama" }

// Use Ollama with a specific model
{ "provider": "ollama", "ollama_model": "mistral" }

// Use Ollama on a different host
{ "provider": "ollama", "ollama_url": "http://192.168.1.100:11434" }
```

---

## Data

knock stores data in `~/.knock/`:

    ~/.knock/
    ├── config.json    # settings
    ├── cache/         # query cache (sled db)
    └── history/       # command history (sled db)

---

## Upgrade

Update to the latest version:

    kn --upgrade

---

## License

MIT
