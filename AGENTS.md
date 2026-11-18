# AGENTS.md

knock (`kn`) is a Rust CLI (edition 2024) that turns a plain-language request into a shell command. All LLM access goes through the sibling crate conduit (`../conduit`); knock owns prompts, output handling, cache and history.

## Commands

```sh
cargo build
cargo test                                   # unit tests only; no network
cargo test store::tests                      # one module
cargo clippy --all-targets
cargo fmt
cargo run -- --provider claude-code --model haiku --no-copy list files   # live smoke test
```

Before finishing any change, run: `cargo fmt && cargo clippy --all-targets && cargo test`

`Cargo.toml` has a `[patch]` that points conduit at `../conduit`. Remove it once the conduit changes are pushed; a plain `cargo install --git` ignores it either way.

## Layout

| Path | What |
|---|---|
| `src/main.rs` | Dispatch, the suggest/explain flows, error reporting (`report`) |
| `src/llm.rs` | Instructions, the per-mode JSON schemas, the `Suggestion` type, `connect` (`--provider`/`--model` overrides) |
| `src/store.rs` | `Cache` and `History` as JSON files in `~/.knock/` |
| `src/context.rs` | OS/shell/cwd detection and piped stdin |
| `src/ui.rs` | Spinner, clipboard, confirmation, running the command |
| `src/shell.rs` | `kn --init` scripts |
| `src/danger.rs` | Local pattern check backing up the model's `dangerous` flag |

## Conventions

- Command modes ask for JSON through conduit's `.schema(...).json()` and deserialize into `Suggestion`. Never parse free text. A new field needs a change to `schema()` and to `Suggestion`, with `#[serde(default)]` so older cached entries still load.
- Anything that changes the answer goes into the cache key (`cache_key` in `main.rs`). Requests with piped stdin are never cached.
- Storage is best-effort. `store.rs` swallows IO and parse errors, and a broken file reads as empty. Never let cache or history failures stop an answer.
- stdout carries only results. Spinner, warnings, prompts and errors go to stderr, so `kn --raw` and pipes stay clean. Confirmation reads from `/dev/tty` when stdin is piped.
- Only read stdin when it's a FIFO or regular file (`stdin_has_input`). Inherited sockets never send EOF and would hang `kn`.
- conduit errors are shown through `report()`, which adds a knock-specific hint (`kn --config`, `--model`). Don't `unwrap`/`expect` on runtime paths.
- Keep knock-specific behavior here. conduit must stay app-agnostic; generic LLM fixes go into conduit.

## Don'ts

- Don't print anything but the command to stdout in `--raw` mode; the shell integration captures it.
- Don't add comments except for non-obvious flow or TODOs.

## Git

Commit or push only when asked. Recent history is mixed; prefer conventional prefixes (`feat:`, `fix:`, `docs:`).
