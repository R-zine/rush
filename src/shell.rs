use crate::history::History;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::env;
use std::io::{self, IsTerminal, Write};
use std::process::Command;

pub fn run() -> io::Result<()> {
    println!("rush - a tiny learning shell");
    println!("Type 'help' for built-ins. Press Ctrl-D to exit.");

    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        run_interactive()
    } else {
        run_piped()
    }
}

fn run_interactive() -> io::Result<()> {
    let mut history = History::new();
    enable_raw_mode()?;

    let result = (|| {
        loop {
            print_prompt()?;
            let Some(line) = read_interactive_line(&mut history)? else {
                break;
            };

            if line.is_empty() {
                continue;
            }
            history.add(&line);
            if process_line(&line)? {
                break;
            }
        }
        Ok(())
    })();

    disable_raw_mode()?;
    result
}

fn run_piped() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print_prompt()?;
        input.clear();

        if stdin.read_line(&mut input)? == 0 {
            break;
        }

        let line = input.trim();
        if line.is_empty() {
            continue;
        }

        if process_line(line)? {
            break;
        }
    }

    Ok(())
}

fn process_line(line: &str) -> io::Result<bool> {
    let command = match parse_command(line) {
        Ok(Some(command)) => command,
        Ok(None) => return Ok(false),
        Err(error) => {
            eprintln!("rush: {error}");
            return Ok(false);
        }
    };

    match execute_builtin(&command)? {
        BuiltinResult::Exit => Ok(true),
        BuiltinResult::Handled => Ok(false),
        BuiltinResult::NotBuiltin => {
            run_external(&command);
            Ok(false)
        }
    }
}

fn read_interactive_line(history: &mut History) -> io::Result<Option<String>> {
    let mut line = String::new();

    loop {
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind,
            ..
        }) = event::read()?
        else {
            continue;
        };

        if !should_process_key_event(kind) {
            continue;
        }

        match (code, modifiers) {
            (KeyCode::Enter, _) => {
                println!();
                return Ok(Some(line));
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) if line.is_empty() => {
                println!();
                return Ok(None);
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                println!("^C");
                return Ok(Some(String::new()));
            }
            (KeyCode::Backspace, _) => {
                line.pop();
                redraw_line(&line)?;
            }
            (KeyCode::Up, _) => {
                line = history.previous(&line);
                redraw_line(&line)?;
            }
            (KeyCode::Down, _) => {
                line = history.next(&line);
                redraw_line(&line)?;
            }
            (KeyCode::Char(character), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                line.push(character);
                print!("{character}");
                io::stdout().flush()?;
            }
            _ => {}
        }
    }
}

fn should_process_key_event(kind: KeyEventKind) -> bool {
    kind == KeyEventKind::Press
}

fn redraw_line(line: &str) -> io::Result<()> {
    print!("\r\x1b[2K");
    print_prompt()?;
    print!("{line}");
    io::stdout().flush()
}

fn print_prompt() -> io::Result<()> {
    let directory = env::current_dir()?;
    print!("rush {}> ", directory.display());
    io::stdout().flush()
}

enum BuiltinResult {
    Handled,
    Exit,
    NotBuiltin,
}

fn execute_builtin(command: &ParsedCommand) -> io::Result<BuiltinResult> {
    match command.program.as_str() {
        "cd" => {
            let destination = command.args.first().map_or_else(
                || env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")),
                |path| Some(path.into()),
            );

            match destination {
                Some(path) => {
                    if let Err(error) = env::set_current_dir(path) {
                        eprintln!("rush: cd: {error}");
                    }
                }
                None => eprintln!("rush: cd: home directory is not set"),
            }
        }
        "exit" | "quit" => return Ok(BuiltinResult::Exit),
        "help" => print_help(),
        "pwd" => println!("{}", env::current_dir()?.display()),
        "echo" => println!("{}", command.args.join(" ")),
        _ => return Ok(BuiltinResult::NotBuiltin),
    }

    Ok(BuiltinResult::Handled)
}

fn print_help() {
    println!("Built-ins:");
    println!("  cd [DIR]  change directory (defaults to your home directory)");
    println!("  echo ...  print arguments");
    println!("  exit      leave rush");
    println!("  help      show this message");
    println!("  pwd       print the working directory");
}

fn run_external(command: &ParsedCommand) {
    match Command::new(&command.program).args(&command.args).status() {
        Ok(status) if !status.success() => {
            eprintln!("rush: process exited with {status}");
        }
        Ok(_) => {}
        Err(error) => eprintln!("rush: {}: {error}", command.program),
    }
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedCommand {
    program: String,
    args: Vec<String>,
}

fn parse_command(line: &str) -> Result<Option<ParsedCommand>, &'static str> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for character in line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        match (quote, character) {
            (Some('"'), '\\') | (None, '\\') => escaped = true,
            (Some(active), character) if active == character => quote = None,
            (None, '\'') | (None, '"') => quote = Some(character),
            (None, character) if character.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            (_, character) => current.push(character),
        }
    }

    if escaped || quote.is_some() {
        return Err("unterminated quote or escape");
    }
    if !current.is_empty() {
        words.push(current);
    }

    Ok(words.split_first().map(|(program, args)| ParsedCommand {
        program: program.clone(),
        args: args.to_vec(),
    }))
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    use super::{ParsedCommand, parse_command, should_process_key_event};

    #[test]
    fn parses_quoted_arguments() {
        assert_eq!(
            parse_command(r#"echo "hello world" 'from rush'"#),
            Ok(Some(ParsedCommand {
                program: "echo".into(),
                args: vec!["hello world".into(), "from rush".into()],
            }))
        );
    }

    #[test]
    fn rejects_unterminated_quotes() {
        assert_eq!(
            parse_command("echo \"unfinished"),
            Err("unterminated quote or escape")
        );
    }

    #[test]
    fn ignores_key_release_events() {
        let press = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE);
        let release = KeyEvent::new_with_kind(
            KeyCode::Char('a'),
            KeyModifiers::NONE,
            KeyEventKind::Release,
        );

        assert!(should_process_key_event(press.kind));
        assert!(!should_process_key_event(release.kind));
    }
}
