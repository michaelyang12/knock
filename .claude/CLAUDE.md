# Claude Code Instructions

**Read these files at the start of each session to understand this project:**

1. **`project.md`** - What Knock does, tech stack, features, providers
2. **`structure.md`** - File layout and module responsibilities
3. **`patterns.md`** - Code conventions and common patterns used

## Quick Reference

- **Language:** Rust 2024 edition
- **Binary:** `kn` (defined in Cargo.toml)
- **Entry point:** `src/main.rs`
- **Build:** `cargo build` / `cargo run`
- **User data:** `~/.knock/` (config, cache, history)

## Key Files for Common Tasks

| Task | Files |
|------|-------|
| Add CLI flag | `src/args.rs` |
| Add new AI provider | `src/client.rs`, `src/config.rs` |
| Change prompt/behavior | `src/client.rs` (system prompt) |
| Modify caching | `src/cache.rs` |
| Change history | `src/history.rs` |
| Setup wizard | `src/setup.rs` |
| Shell detection | `src/context.rs` |
