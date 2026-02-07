# regular-expression

A regular-expression engine and grep-like CLI written in Rust.

This workspace contains:
- `regex-core`: parser/compiler/evaluator library.
- `regex-cli`: command-line tool (`regex`) built on top of `regex-core`.

## Installation

```sh
cargo install --git https://github.com/shu-kitamura/regular-expression.git -p regex-cli
```

## CLI Usage

```text
regex [OPTIONS] PATTERN [FILE...]
regex [OPTIONS] -e PATTERN [-e PATTERN ...] [FILE...]
command | regex [OPTIONS] PATTERN
command | regex [OPTIONS] -e PATTERN [-e PATTERN ...]
```

If no file is given, input is read from stdin.

## Options

- `-e, --regexp <PATTERN>`: Add a pattern. Can be specified multiple times.
- `-c, --count`: Print only the number of matching lines.
- `-i, --ignore-case`: Case-insensitive matching.
- `-v, --invert-match`: Select non-matching lines.
- `-h, --no-filename`: Never print file names in output.
- `-H, --with-filename`: Always print file names in output.
- `-n, --line-number`: Prefix each output line with its line number.
- `--help`: Show help.
- `-V, --version`: Show version.

Notes:
- `-h` and `-H` cannot be used together.
- With multiple input files, file names are shown by default.
- With one input file (or stdin), file names are hidden by default.

## Supported Regex Syntax

- Literals (e.g. `abc`)
- Escaped literals (e.g. `\*`, `\+`, `\\`)
- Wildcard: `.`
- Character classes: `[abc]`, ranges `[a-z]`, negated classes `[^0-9]`
- Quantifiers: `*`, `+`, `?`, `{m}`, `{m,}`, `{m,n}`
- Grouping and alternation: `(ab|cd)`
- Captures and backreferences: `(abc)\1`
- Anchors: `^` and `$`

Current limitations:
- Non-greedy quantifiers (`*?`, `+?`, `??`, `{m,n}?`) are not supported.
- Non-capturing groups (`(?:...)`) are not supported.
- Escape sequences like `\d` and `\w` are treated as literal characters (`d`, `w`), not special classes.

## Examples

```sh
# Search in a file
regex "ho*" test.txt

# Case-insensitive search
regex -i "ho*" test.txt

# Count matches
regex -c "ho*" test.txt

# Invert matches
regex -v "ho*" test.txt

# Multiple patterns (OR)
regex -e "ho*" -e "fu*" test.txt

# Read from stdin
cat test.txt | regex -e "ho*" -e "fu*"

# Use captures + backreference
regex "(abc)\\1" test.txt
```

## Library Usage (`regex-core`)

```rust
use regex_core::Regex;

fn main() -> Result<(), regex_core::error::RegexError> {
    let re = Regex::new("(abc)\\1", false, false)?;
    assert!(re.is_match("abcabc")?);
    assert!(!re.is_match("abcabd")?);
    Ok(())
}
```

## Development Commands

```sh
# Build
cargo build

# Run CLI
cargo run -p regex-cli --bin regex -- "a*b" sample.txt

# Test
cargo test

# Lint
cargo clippy --workspace -- -D warnings

# Format
cargo fmt --all

# Benchmark (criterion)
cargo bench -p regex-core
```

## License

This project is licensed under the MIT License.  
See `LICENCE.txt` for details.
