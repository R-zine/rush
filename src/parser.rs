#[derive(Debug, PartialEq, Eq)]
pub struct ParsedCommand {
    pub program: String,
    pub args: Vec<String>,
}

pub fn parse_command(line: &str) -> Result<Option<ParsedCommand>, &'static str> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut token_started = false;

    for character in line.chars() {
        if escaped {
            current.push(character);
            escaped = false;
            continue;
        }

        match (quote, character) {
            (Some('"'), '\\') | (None, '\\') => {
                escaped = true;
                token_started = true;
            }
            (Some(active), character) if active == character => quote = None,
            (None, '\'') | (None, '"') => {
                quote = Some(character);
                token_started = true;
            }
            (None, character) if character.is_whitespace() => {
                if token_started {
                    words.push(std::mem::take(&mut current));
                    token_started = false;
                }
            }
            (_, character) => {
                current.push(character);
                token_started = true;
            }
        }
    }

    if escaped || quote.is_some() {
        return Err("unterminated quote or escape");
    }
    if token_started {
        words.push(current);
    }

    Ok(words.split_first().map(|(program, args)| ParsedCommand {
        program: program.clone(),
        args: args.to_vec(),
    }))
}

#[cfg(test)]
mod tests {
    use super::{ParsedCommand, parse_command};

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
    fn preserves_empty_quoted_arguments() {
        assert_eq!(
            parse_command(r#"echo before "" '' after"#),
            Ok(Some(ParsedCommand {
                program: "echo".into(),
                args: vec!["before".into(), "".into(), "".into(), "after".into()],
            }))
        );
    }

    #[test]
    fn concatenates_quoted_and_unquoted_segments() {
        assert_eq!(
            parse_command(r#"echo one" two" "three"four''"#),
            Ok(Some(ParsedCommand {
                program: "echo".into(),
                args: vec!["one two".into(), "threefour".into()],
            }))
        );
    }

    #[test]
    fn parses_backslash_escapes() {
        assert_eq!(
            parse_command(r#"echo escaped\ space "quoted\"value" 'literal\value'"#),
            Ok(Some(ParsedCommand {
                program: "echo".into(),
                args: vec![
                    "escaped space".into(),
                    "quoted\"value".into(),
                    "literal\\value".into(),
                ],
            }))
        );
    }

    #[test]
    fn ignores_separator_whitespace() {
        assert_eq!(
            parse_command("  echo\tvalue  "),
            Ok(Some(ParsedCommand {
                program: "echo".into(),
                args: vec!["value".into()],
            }))
        );
        assert_eq!(parse_command(" \t "), Ok(None));
    }

    #[test]
    fn rejects_unterminated_quotes() {
        assert_eq!(
            parse_command("echo \"unfinished"),
            Err("unterminated quote or escape")
        );
    }

    #[test]
    fn rejects_unterminated_escape() {
        assert_eq!(
            parse_command("echo unfinished\\"),
            Err("unterminated quote or escape")
        );
    }
}
