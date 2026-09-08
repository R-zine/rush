use std::io::Write;
use std::process::{Command, Output, Stdio};

fn run_shell(input: &str) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_rush"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to start rush");

    child
        .stdin
        .take()
        .expect("rush stdin was not piped")
        .write_all(input.as_bytes())
        .expect("failed to write shell input");

    child.wait_with_output().expect("failed to wait for rush")
}

fn successful_shell(input: &str) -> (String, String) {
    let output = run_shell(input);
    assert!(
        output.status.success(),
        "rush failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (
        String::from_utf8(output.stdout)
            .expect("rush stdout was not valid UTF-8")
            .replace("\r\n", "\n"),
        String::from_utf8(output.stderr)
            .expect("rush stderr was not valid UTF-8")
            .replace("\r\n", "\n"),
    )
}

#[test]
fn runs_all_built_in_commands() {
    let initial_directory = std::env::current_dir().expect("failed to read current directory");
    let parent_directory = initial_directory
        .parent()
        .expect("test directory had no parent");
    let (stdout, stderr) =
        successful_shell("help\npwd\necho integration works\ncd ..\npwd\nquit\n");

    let expected = format!(
        "Built-ins:\n\
         \x20 cd [DIR]  change directory (defaults to your home directory)\n\
         \x20 echo ...  print arguments\n\
         \x20 exit      leave rush\n\
         \x20 help      show this message\n\
         \x20 pwd       print the working directory\n\
         {}\n\
         integration works\n\
         {}\n",
        initial_directory.display(),
        parent_directory.display()
    );
    assert_eq!(stdout, expected);
    assert!(stderr.is_empty());
}

#[test]
fn exit_alias_terminates_shell() {
    let (stdout, stderr) = successful_shell("echo before exit\nexit\necho after exit\n");

    assert_eq!(stdout, "before exit\n");
    assert!(stderr.is_empty());
}

#[test]
fn runs_external_command() {
    #[cfg(windows)]
    let input = "cmd /C echo external works\nexit\n";
    #[cfg(not(windows))]
    let input = "sh -c 'printf external works'\nexit\n";

    let (stdout, stderr) = successful_shell(input);

    assert_eq!(stdout, "external works\n");
    assert!(stderr.is_empty());
}

#[test]
fn piped_mode_emits_only_command_output() {
    let (stdout, stderr) = successful_shell("echo clean output\n");

    assert_eq!(stdout, "clean output\n");
    assert!(stderr.is_empty());
    assert!(!stdout.contains("rush -"));
    assert!(!stdout.contains("> "));
}

#[test]
fn preserves_empty_quoted_arguments() {
    let (stdout, stderr) = successful_shell("echo before \"\" '' after\nexit\n");

    assert_eq!(stdout, "before   after\n");
    assert!(stderr.is_empty());
}

#[test]
fn preserves_an_escaped_trailing_space() {
    let (stdout, stderr) = successful_shell("echo trailing\\ \nexit\n");

    assert_eq!(stdout, "trailing \n");
    assert!(stderr.is_empty());
}

#[test]
fn reports_a_parse_error_and_continues() {
    let (stdout, stderr) = successful_shell("echo \"unfinished\necho recovered\nexit\n");

    assert_eq!(stdout, "recovered\n");
    assert_eq!(stderr, "rush: unterminated quote or escape\n");
}

#[test]
fn reports_an_unknown_program_and_continues() {
    let missing_program = "rush-command-that-does-not-exist";
    let (stdout, stderr) = successful_shell(&format!("{missing_program}\necho recovered\nexit\n"));

    assert_eq!(stdout, "recovered\n");
    assert!(stderr.starts_with(&format!("rush: {missing_program}:")));
}
