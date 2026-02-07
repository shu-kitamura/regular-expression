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
- Full `egrep` compatibility
- Large feature changes without an issue/spec discussion first

## 2. Repository / Crate Layout

- `crates/regex-core/src/lib.rs`: Public library API for the regex engine.
- `crates/regex-core/src/engine/`: Core implementation
  - `ast.rs`, `parser.rs`, `compiler.rs`, `evaluator.rs`, `instruction.rs`.
- `crates/regex-core/benches/regex_engine_bench.rs`: Criterion benchmarks (`cargo bench`).
- `crates/regex-cli/src/main.rs`: CLI entry (built as `regex`).
- `crates/regex-cli/tests/`: CLI integration tests (executes `cargo run`).
- `crates/regex-cli/src/tests/`: CLI unit tests for helper functions in `main.rs`.
- `Cargo.toml`: Workspace root (members + shared version/edition).
- `crates/*/Cargo.toml`: Per-crate dependencies and bin config.

## 3. Development Commands

- Build: `cargo build` or `cargo build -p regex-core` / `cargo build -p regex-cli`.
- Run CLI (file): `cargo run -p regex-cli --bin regex -- "a*b" input.txt`.
- Run CLI (stdin): `echo "aaab" | cargo run -p regex-cli --bin regex -- "a*b"`.
- Tests: `cargo test` or `cargo test -p regex-core` / `cargo test -p regex-cli`.
- Benchmarks: `cargo bench -p regex-core`.
- Lint: `cargo clippy --workspace -- -D warnings` — keep warnings at zero.
- Format: `cargo fmt --all` — apply standard Rust formatting.

## 4. Coding Style & Naming Conventions

- Rust 2024, 4-space indent; use `cargo fmt` before pushing.
- Names: `snake_case` for functions/modules, `CamelCase` for types, `SCREAMING_SNAKE_CASE` for consts.
- Errors via `thiserror`; avoid `unwrap`/`expect` in library code.
- Public API lives in `crates/regex-core/src/lib.rs`; keep `crates/regex-core/src/engine/*` focused and single‑responsibility.

## 5. Testing Guidelines

- Framework: Rust built-in test harness.
- Locations: unit tests near code; CLI integration tests under `crates/regex-cli/tests/`; CLI helper tests under `crates/regex-cli/src/tests/`.
- Filenames: end with `_tests.rs` (e.g., `integration_tests.rs`).
- Run subsets: `cargo test engine::parser` or `cargo test compile_patterns`.

## 6. Commit & Pull Request Guidelines

- Style: imperative mood, concise scope. Prefer Conventional Commits when possible
  (e.g., `feat(engine): add repetition operator`, `fix: clippy warning`).
- Include: what/why, any behavior changes, and test coverage notes.
- PRs: link issues, show `cargo test` output or describe manual CLI checks
  (example: `cargo run -p regex-cli --bin regex -- "[a-z]+" sample.txt -> matched`).

## 7. Security & Configuration Tips
- No network or unsafe code expected; keep dependencies minimal.
- Validate and sanitize CLI inputs; prefer clear error messages over panics.
- Keep `clap` argument parsing in `crates/regex-cli/src/main.rs`; core engine should remain I/O‑free.
