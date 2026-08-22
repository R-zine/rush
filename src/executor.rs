use crate::parser::ParsedCommand;
use std::env;
use std::io;
use std::process::Command;

pub enum ExecutionResult {
    Handled,
    Exit,
    NotBuiltin,
}

pub fn execute(command: &ParsedCommand) -> io::Result<ExecutionResult> {
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
        "exit" | "quit" => return Ok(ExecutionResult::Exit),
        "help" => print_help(),
        "pwd" => println!("{}", env::current_dir()?.display()),
        "echo" => println!("{}", command.args.join(" ")),
        _ => return Ok(ExecutionResult::NotBuiltin),
    }

    Ok(ExecutionResult::Handled)
}

pub fn execute_external(command: &ParsedCommand) {
    match Command::new(&command.program).args(&command.args).status() {
        Ok(status) if !status.success() => {
            eprintln!("rush: process exited with {status}");
        }
        Ok(_) => {}
        Err(error) => eprintln!("rush: {}: {error}", command.program),
    }
}

fn print_help() {
    println!("Built-ins:");
    println!("  cd [DIR]  change directory (defaults to your home directory)");
    println!("  echo ...  print arguments");
    println!("  exit      leave rush");
    println!("  help      show this message");
    println!("  pwd       print the working directory");
}
