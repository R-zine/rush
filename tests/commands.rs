use std::io::Write;
use std::process::{Command, Stdio};

fn run_shell(input: &str) -> String {
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

    let output = child.wait_with_output().expect("failed to wait for rush");
    assert!(
        output.status.success(),
        "rush failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("rush output was not valid UTF-8")
}

#[test]
fn runs_all_built_in_commands() {
    let output = run_shell("help\npwd\necho integration works\ncd .\npwd\nquit\n");

    assert!(output.contains("Built-ins:"));
    assert!(output.contains("cd [DIR]"));
    assert!(output.contains("integration works"));
    assert!(output.matches("rush ").count() >= 5);
    assert!(output.contains(&format!(
        "rush {}>",
        std::env::current_dir().unwrap().display()
    )));
}

#[test]
fn exit_alias_terminates_shell() {
    let output = run_shell("echo before exit\nexit\necho after exit\n");

    assert!(output.contains("before exit"));
    assert!(!output.contains("after exit"));
}

#[test]
fn runs_external_command() {
    #[cfg(windows)]
    let input = "cmd /C echo external works\nexit\n";
    #[cfg(not(windows))]
    let input = "sh -c 'printf external works'\nexit\n";

    let output = run_shell(input);

    assert!(output.contains("external works"));
}
