use crate::executor::{self, ExecutionResult};
use crate::history::History;
use crate::parser::parse_command;
use crossterm::QueueableCommand;
use crossterm::cursor::MoveToColumn;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use crossterm::terminal::{Clear, ClearType, disable_raw_mode, enable_raw_mode};
use std::env;
use std::io::{self, IsTerminal, Write};

pub fn run() -> io::Result<()> {
    if io::stdin().is_terminal() && io::stdout().is_terminal() {
        println!("rush - a tiny learning shell");
        println!("Type 'help' for built-ins. Press Ctrl-D to exit.");
        run_interactive()
    } else {
        run_piped()
    }
}

fn run_interactive() -> io::Result<()> {
    let mut history = History::new();

    loop {
        print_prompt()?;
        let Some(line) = read_interactive_line(&mut history)? else {
            break;
        };

        history.add(&line);
        if line.is_empty() {
            continue;
        }
        if process_line(&line)? {
            break;
        }
    }

    Ok(())
}

fn run_piped() -> io::Result<()> {
    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        input.clear();

        if stdin.read_line(&mut input)? == 0 {
            break;
        }

        let line = trim_line_ending(&input);
        if line.is_empty() {
            continue;
        }

        if process_line(line)? {
            break;
        }
    }

    Ok(())
}

fn trim_line_ending(input: &str) -> &str {
    let input = input.strip_suffix('\n').unwrap_or(input);
    input.strip_suffix('\r').unwrap_or(input)
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
    let mut raw_mode = RawModeGuard::enter()?;
    let mut stdout = io::stdout();
    let line = read_interactive_line_from(history, &mut stdout, event::read);
    let restore_result = raw_mode.restore();

    match line {
        Ok(line) => {
            restore_result?;
            Ok(line)
        }
        Err(error) => Err(error),
    }
}

fn read_interactive_line_from<W, F>(
    history: &mut History,
    output: &mut W,
    mut read_event: F,
) -> io::Result<Option<String>>
where
    W: Write,
    F: FnMut() -> io::Result<Event>,
{
    let mut line = String::new();

    loop {
        let Event::Key(KeyEvent {
            code,
            modifiers,
            kind,
            ..
        }) = read_event()?
        else {
            continue;
        };

        if !should_process_key_event(kind) {
            continue;
        }

        match (code, modifiers) {
            (KeyCode::Enter, _) => {
                write!(output, "\r\n")?;
                output.flush()?;
                return Ok(Some(line));
            }
            (KeyCode::Char('d'), KeyModifiers::CONTROL) if line.is_empty() => {
                write!(output, "\r\n")?;
                output.flush()?;
                return Ok(None);
            }
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                write!(output, "^C\r\n")?;
                output.flush()?;
                return Ok(Some(String::new()));
            }
            (KeyCode::Backspace, _) => {
                line.pop();
                redraw_line(output, &line)?;
            }
            (KeyCode::Up, _) => {
                line = history.previous(&line);
                redraw_line(output, &line)?;
            }
            (KeyCode::Down, _) => {
                line = history.next(&line);
                redraw_line(output, &line)?;
            }
            (KeyCode::Char(character), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                line.push(character);
                write!(output, "{character}")?;
                output.flush()?;
            }
            _ => {}
        }
    }
}

fn should_process_key_event(kind: KeyEventKind) -> bool {
    matches!(kind, KeyEventKind::Press | KeyEventKind::Repeat)
}

fn redraw_line<W: Write>(output: &mut W, line: &str) -> io::Result<()> {
    output.queue(MoveToColumn(0))?;
    output.queue(Clear(ClearType::CurrentLine))?;
    write_prompt(output)?;
    write!(output, "{line}")?;
    output.flush()
}

fn print_prompt() -> io::Result<()> {
    let mut stdout = io::stdout();
    write_prompt(&mut stdout)?;
    stdout.flush()
}

fn write_prompt<W: Write>(output: &mut W) -> io::Result<()> {
    let directory = env::current_dir()?;
    write!(output, "rush {}> ", directory.display())
}

struct RawModeGuard {
    active: bool,
}

impl RawModeGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        Ok(Self { active: true })
    }

    fn restore(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        self.active = false;
        Ok(())
    }
}

impl Drop for RawModeGuard {
    fn drop(&mut self) {
        if self.active {
            let _ = disable_raw_mode();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    use super::{read_interactive_line_from, should_process_key_event, trim_line_ending};
    use crate::history::History;

    fn read_line(events: Vec<Event>, history: &mut History) -> (Option<String>, String) {
        let mut events = VecDeque::from(events);
        let mut output = Vec::new();
        let line = read_interactive_line_from(history, &mut output, || {
            Ok(events.pop_front().expect("test ran out of events"))
        })
        .expect("line editor failed");

        (
            line,
            String::from_utf8(output).expect("output was not UTF-8"),
        )
    }

    fn key(code: KeyCode) -> Event {
        Event::Key(KeyEvent::new(code, KeyModifiers::NONE))
    }

    fn control(character: char) -> Event {
        Event::Key(KeyEvent::new(
            KeyCode::Char(character),
            KeyModifiers::CONTROL,
        ))
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

    #[test]
    fn processes_key_repeat_events() {
        assert!(should_process_key_event(KeyEventKind::Repeat));
    }

    #[test]
    fn reads_characters_and_applies_backspace() {
        let events = vec![
            key(KeyCode::Char('a')),
            key(KeyCode::Char('b')),
            key(KeyCode::Backspace),
            key(KeyCode::Char('c')),
            key(KeyCode::Enter),
        ];

        let (line, output) = read_line(events, &mut History::new());

        assert_eq!(line.as_deref(), Some("ac"));
        assert!(output.ends_with("ac\r\n"));
    }

    #[test]
    fn navigates_history_and_restores_draft() {
        let mut history = History::new();
        history.add("first");
        history.add("second");
        let events = vec![
            key(KeyCode::Char('d')),
            key(KeyCode::Up),
            key(KeyCode::Up),
            key(KeyCode::Down),
            key(KeyCode::Down),
            key(KeyCode::Enter),
        ];

        let (line, _) = read_line(events, &mut history);

        assert_eq!(line.as_deref(), Some("d"));
    }

    #[test]
    fn control_c_cancels_the_current_line() {
        let events = vec![key(KeyCode::Char('x')), control('c')];

        let (line, output) = read_line(events, &mut History::new());

        assert_eq!(line.as_deref(), Some(""));
        assert!(output.ends_with("^C\r\n"));
    }

    #[test]
    fn control_d_exits_only_on_an_empty_line() {
        let (line, _) = read_line(vec![control('d')], &mut History::new());
        assert_eq!(line, None);

        let events = vec![key(KeyCode::Char('x')), control('d'), key(KeyCode::Enter)];
        let (line, _) = read_line(events, &mut History::new());
        assert_eq!(line.as_deref(), Some("x"));
    }

    #[test]
    fn strips_only_the_line_ending() {
        assert_eq!(trim_line_ending("echo value\n"), "echo value");
        assert_eq!(trim_line_ending("echo value\r\n"), "echo value");
        assert_eq!(trim_line_ending("echo value "), "echo value ");
        assert_eq!(trim_line_ending("echo value\r"), "echo value");
    }
}
