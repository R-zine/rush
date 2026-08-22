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
    fn rejects_unterminated_quotes() {
        assert_eq!(
            parse_command("echo \"unfinished"),
            Err("unterminated quote or escape")
        );
    }
}
