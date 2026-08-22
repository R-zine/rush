# rush

`rush` is a small Unix-like command shell for learning Rust systems programming.
It uses the small `crossterm` crate for cross-platform terminal key events.

## Run it

```text
cargo run
```

Try these commands:

```text
help
pwd
echo "hello from rush"
cd ..
exit
```

In an interactive terminal, use Up and Down to navigate command history.
Backspace edits the current line, Ctrl-C cancels it, and Ctrl-D exits when the
line is empty. History is kept in memory for the current session.

## What it teaches

The implementation is organized as a simple loop:

1. Print a prompt based on the current directory.
2. Read interactive keys, including Up and Down for history navigation.
3. Parse whitespace, quotes, and backslash escapes.
4. Handle built-in commands in the current process.
5. Start other programs with `std::process::Command`.

The code follows those responsibilities across focused modules:

- `parser.rs` converts input text into parsed commands.
- `executor.rs` runs built-ins and external programs.
- `history.rs` stores commands and manages Up/Down navigation.
- `shell.rs` handles terminal input and coordinates the modules.

Up recalls older commands, Down moves forward again, and the original draft is
restored at the end. Consecutive duplicate commands are not stored.

Suggested next exercises:

- Support `>` output redirection and `<` input redirection.
- Add pipelines such as `echo hello | findstr hello` on Windows.
- Add environment-variable expansion.

## Verify

```text
cargo fmt --check
cargo test
cargo clippy -- -D warnings
```
