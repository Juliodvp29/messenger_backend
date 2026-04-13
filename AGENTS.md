# AGENTS.md

## Build & Run

- Requires **nightly Rust** (edition = "2024" in Cargo.toml)
- `cargo run` — compile and run
- `cargo check` — typecheck only
- `cargo build` — build debug binary

## Project Structure

- `src/main.rs` — entry point (currently minimal)
- `mainchatdb.sql` — complete database schema (PostgreSQL)

## Key Files

- `Cargo.toml` — package manifest
- `mainchatdb.sql` — defines all tables, functions, indexes, triggers, RLS policies for a WhatsApp/Telegram-style messaging app

## Notes

- The SQL schema is the most complete piece of documentation for this project
- No tests, CI, or lint config exists yet
