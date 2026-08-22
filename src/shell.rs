use crate::executor::{self, ExecutionResult};
use crate::history::History;
use crate::parser::parse_command;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use std::env;
use std::io::{self, IsTerminal, Write};

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

    match executor::execute(&command)? {
        ExecutionResult::Exit => Ok(true),
        ExecutionResult::Handled => Ok(false),
        ExecutionResult::NotBuiltin => {
            executor::execute_external(&command);
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

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    use super::should_process_key_event;

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
