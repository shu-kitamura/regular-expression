# AGENTS.md

This file is a guide for agents (humans/AI) working on this repository.
Goal: make changes quickly and safely with minimal back-and-forth.

---

## 1. Purpose & Scope

This repository provides:
- A regular expression engine (library)
- A CLI built on top of the engine (I/O + formatting)

Primary goals:
- Correctness and reproducibility (behavior is defined by tests)
- Avoid performance regressions (benchmarks when relevant)

Non-goals (adjust as needed):
- Full PCRE compatibility
- Large feature changes without an issue/spec discussion first

## 2. Repository / Crate Layout

- `src/lib.rs`: Public library API for the regex engine.
- `src/engine/`: Core implementation
  - `parser.rs`, `compiler.rs`, `evaluator.rs`, `instruction.rs`.
- `src/bin/regex.rs`: CLI entry (built as `regex`).
- `tests/`: Integration tests (engine-level).
- `src/bin/tests/`: Integration tests for the CLI.
- `Cargo.toml`: Dependencies (`clap`, `thiserror`) and bin config.

## 3. Development Commands

- Build: `cargo build` — compile library and binary.
- Run CLI: `cargo run -- "a*b" "aaab"` — example pattern + input.
- Tests: `cargo test` — run unit + integration tests.
- Lint: `cargo clippy -- -D warnings` — keep warnings at zero.
- Format: `cargo fmt --all` — apply standard Rust formatting.

## 4. Coding Style & Naming Conventions

- Rust 2021, 4-space indent; use `cargo fmt` before pushing.
- Names: `snake_case` for functions/modules, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for consts.
- Errors via `thiserror`; avoid `unwrap`/`expect` in library code.
- Public API lives in `lib.rs`; keep `src/engine/*` focused and single‑responsibility.

## 5. Testing Guidelines

- Framework: Rust built-in test harness.
- Locations: unit tests near code; integration tests under `tests/` and `src/bin/tests/`.
- Filenames: end with `_tests.rs` (e.g., `integration_tests.rs`).
- Run subsets: `cargo test engine::parser` or `cargo test compile_patterns`.

## 6. Commit & Pull Request Guidelines

- Style: imperative mood, concise scope. Prefer Conventional Commits when possible
  (e.g., `feat(engine): add repetition operator`, `fix: clippy warning`).
- Include: what/why, any behavior changes, and test coverage notes.
- PRs: link issues, show `cargo test` output or describe manual CLI checks
  (example: `cargo run -- "[a-z]+" "hello" -> matched`).

## 7. Security & Configuration Tips
- No network or unsafe code expected; keep dependencies minimal.
- Validate and sanitize CLI inputs; prefer clear error messages over panics.
- Keep `clap` argument parsing in `src/bin/regex.rs`; core engine should remain I/O‑free.
