# knock

Describe a shell command in plain language, get the command back.

```console
$ kn find files over 500MB here
find . -type f -size +500M
copied to clipboard
```

## Install

```sh
cargo install --git https://github.com/michaelyang12/knock.git --locked
```

This installs `kn` into `~/.cargo/bin`. Upgrade later with `kn --upgrade`.

## Setup

```sh
kn --config
```

Choose a provider and model: each provider shows whether it is ready, models are listed live, and Esc goes back a step. Supported providers include:

| Provider | Needs |
|---|---|
| OpenAI, Anthropic, Gemini, Groq, OpenRouter, DeepSeek | the provider's API key in your environment, e.g. `OPENAI_API_KEY` |
| Claude Code, Codex | the `claude` / `codex` CLI, signed in (subscriptions work) |
| Ollama, LM Studio | the local server running |
| Any OpenAI-compatible server | its URL |

Config lives in `~/.config/conduit/knock/config.toml`. LLM access goes through [conduit](https://github.com/michaelyang12/conduit), which documents the config format.

## Usage

```sh
kn undo my last commit but keep the changes     # quotes optional
kn -v list docker containers                    # explain the command
kn -a compress this folder                      # alternatives
kn -x delete node_modules everywhere            # run it after confirming
kn explain tar -xzvf archive.tgz                # explain an existing command
cargo build 2>&1 | kn fix this                  # piped output becomes context
kn --history                                    # recent commands
kn --history git                                # search history
```

| Flag | |
|---|---|
| `-v`, `--verbose` | explain the command |
| `-a`, `--alt` | show alternatives |
| `-x`, `--execute` | run it after a `[y/N]` confirmation |
| `-f`, `--fresh` | skip the cache |
| `--provider NAME`, `--model NAME` | override the provider or model for one run |
| `--no-copy` | don't copy to the clipboard |
| `--raw` | print only the command (also the default when output is piped) |

Put flags before `explain`: anything after it is treated as the command to explain.

Commands that delete data, need `sudo`, rewrite git history and so on get a red warning. With `-x` they ask "Run it anyway?". Both the model and a local pattern check can flag a command.

## Shell integration

Put generated commands straight onto your prompt line, ready to edit, instead of only on the clipboard:

```sh
eval "$(kn --init zsh)"     # ~/.zshrc
eval "$(kn --init bash)"    # ~/.bashrc (bash 4+)
kn --init fish | source     # ~/.config/fish/config.fish
```

With integration on, `kn <query>` fills your next prompt. Calls that start with a flag, and `kn explain`, behave as before.

## Data

```text
~/.knock/
├── cache.json     # answers per query, provider and model, kept 30 days
└── history.json   # last 200 commands
```

Both files are optional and safe to delete. knock 0.2 kept this data in sled databases (`~/.knock/cache/`, `~/.knock/history/`); 0.3 doesn't read them, so you can delete those folders too.

## License

MIT
