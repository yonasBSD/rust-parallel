use std::process::Command;

use assert_cmd::cargo::CommandCargoExt;

use predicates::prelude::*;

fn rust_parallel_raw_command() -> Command {
    let mut cmd = Command::cargo_bin("rust-parallel").unwrap();
    cmd.current_dir("tests/");
    cmd
}

fn rust_parallel() -> assert_cmd::Command {
    assert_cmd::Command::from_std(rust_parallel_raw_command())
}

#[test]
fn runs_successfully() {
    rust_parallel()
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_commands_from_args() {
    rust_parallel()
        .arg("echo")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .success()
        .stdout(
            (predicate::str::contains("\n").count(3))
                .and(predicate::str::contains("A\n").count(1))
                .and(predicate::str::contains("B\n").count(1))
                .and(predicate::str::contains("C\n").count(1)),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_commands_from_args_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("echo")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .success()
        .stdout(predicate::eq("A\nB\nC\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_commands_from_args_keep_order() {
    rust_parallel()
        .arg("-k")
        .arg("echo")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .success()
        .stdout(
            (predicate::str::contains("\n").count(3))
                .and(predicate::str::contains("A\n").count(1))
                .and(predicate::str::contains("B\n").count(1))
                .and(predicate::str::contains("C\n").count(1)),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_keep_order_with_sleep() {
    // This test uses sleep commands with different durations
    // Without -k, the output would be in the order of completion (C, B, A)
    // With -k, the output should be in the order of input (A, B, C)
    rust_parallel()
        .arg("-k")
        .arg("-s")
        .arg(r#"sleep {1}; echo {1}"#)
        .arg(":::")
        .arg("0.3")
        .arg("0.2")
        .arg("0.1")
        .assert()
        .success()
        .stdout(predicate::eq("0.3\n0.2\n0.1\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_commands_dry_run() {
    rust_parallel()
        .arg("-s")
        .arg("--dry-run")
        .arg("echo")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .success()
        .stdout(
            (predicate::str::contains("\n").count(3))
                .and(
                    predicate::str::contains(
                        r#"cmd="/bin/bash",args=["-c", "echo A"],line=command_line_args:1"#,
                    )
                    .count(1),
                )
                .and(
                    predicate::str::contains(
                        r#"cmd="/bin/bash",args=["-c", "echo B"],line=command_line_args:2"#,
                    )
                    .count(1),
                )
                .and(
                    predicate::str::contains(
                        r#"cmd="/bin/bash",args=["-c", "echo C"],line=command_line_args:3"#,
                    )
                    .count(1),
                ),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn timeout_sleep_commands_from_args() {
    rust_parallel()
        .arg("-t1")
        .arg("sleep")
        .arg(":::")
        .arg("0")
        .arg("5")
        .assert()
        .failure()
        .code(1)
        .stdout(
            (predicate::str::contains("timeout: deadline has elapsed").count(1))
                .and(predicate::str::contains("timeouts=1").count(1)),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_stdin() {
    let stdin = r#"
        echo A
        echo B
        echo C
    "#;
    rust_parallel()
        .write_stdin(stdin)
        .assert()
        .success()
        .stdout(
            (predicate::str::contains("\n").count(3))
                .and(predicate::str::contains("A\n").count(1))
                .and(predicate::str::contains("B\n").count(1))
                .and(predicate::str::contains("C\n").count(1)),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_echo_stdin_j1() {
    let stdin = r#"
        echo A
        echo B
        echo C
    "#;
    rust_parallel()
        .arg("-j1")
        .write_stdin(stdin)
        .assert()
        .success()
        .stdout(predicate::eq("A\nB\nC\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_file() {
    rust_parallel()
        .arg("-i")
        .arg("file.txt")
        .arg("echo")
        .assert()
        .success()
        .stdout(
            (predicate::str::contains("\n").count(4))
                .and(predicate::str::contains("hello\n").count(1))
                .and(predicate::str::contains("from\n").count(1))
                .and(predicate::str::contains("input\n").count(1))
                .and(predicate::str::contains("file\n").count(1)),
        )
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_file_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("-i")
        .arg("file.txt")
        .arg("echo")
        .assert()
        .success()
        .stdout(predicate::eq("hello\nfrom\ninput\nfile\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn fails_j0() {
    rust_parallel()
        .arg("-j0")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "invalid value '0' for '--jobs <JOBS>'",
        ));
}

#[test]
fn fails_t0() {
    rust_parallel()
        .arg("-t0")
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains(
            "invalid value '0' for '--timeout-seconds <TIMEOUT_SECONDS>'",
        ));
}

#[test]
fn runs_shell_function_from_stdin_j1() {
    let stdin = r#"A
        B
        C"#;

    rust_parallel()
        .write_stdin(stdin)
        .arg("-j1")
        .arg("-s")
        .arg("--shell-path=./dummy_shell.sh")
        .arg("shell_function")
        .assert()
        .success()
        .stdout(predicate::eq(
            "dummy_shell arg1=-c arg2=shell_function A\ndummy_shell arg1=-c arg2=shell_function B\ndummy_shell arg1=-c arg2=shell_function C\n",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_shell_function_from_file_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("-i")
        .arg("file.txt")
        .arg("-s")
        .arg("--shell-path=./dummy_shell.sh")
        .arg("shell_function")
        .assert()
        .success()
        .stdout(predicate::eq(
            "dummy_shell arg1=-c arg2=shell_function hello\ndummy_shell arg1=-c arg2=shell_function from\ndummy_shell arg1=-c arg2=shell_function input\ndummy_shell arg1=-c arg2=shell_function file\n",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_shell_function_from_args_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("-s")
        .arg("--shell-path=./dummy_shell.sh")
        .arg("shell_function")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .success()
        .stdout(predicate::eq(
            "dummy_shell arg1=-c arg2=shell_function A\ndummy_shell arg1=-c arg2=shell_function B\ndummy_shell arg1=-c arg2=shell_function C\n",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_regex_from_input_file_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("-i")
        .arg("csv_file.txt")
        .arg("-r")
        .arg("(?P<arg1>.*),(?P<arg2>.*),(?P<arg3>.*)")
        .arg("echo")
        .arg("arg1={arg1}")
        .arg("arg2={arg2}")
        .arg("arg3={arg3}")
        .arg("dollarzero={0}")
        .arg("emptygroup={}")
        .assert()
        .success()
        .stdout(predicate::eq(
            "arg1=1 arg2=2 arg3=3 dollarzero=1,2,3 emptygroup=1,2,3\narg1=foo arg2=bar arg3=baz dollarzero=foo,bar,baz emptygroup=foo,bar,baz\n",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_regex_from_input_file_badline_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("-i")
        .arg("csv_file_badline.txt")
        .arg("-r")
        .arg("(?P<arg1>.*),(?P<arg2>.*),(?P<arg3>.*)")
        .arg("echo")
        .arg("arg1={arg1}")
        .arg("arg2={arg2}")
        .arg("arg3={arg3}")
        .arg("dollarzero={0}")
        .assert()
        .success()
        .stdout((predicate::str::contains("\n").count(3)).and(predicate::str::contains(
                "regex did not match input data: badline\n").and(
                    predicate::str::contains(
                        "arg1=1 arg2=2 arg3=3 dollarzero=1,2,3\narg1=foo arg2=bar arg3=baz dollarzero=foo,bar,baz\n",
                    )
                )
            ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_regex_from_command_line_args_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("-r")
        .arg("(.*),(.*),(.*)")
        .arg("echo")
        .arg("arg1={1}")
        .arg("arg2={2}")
        .arg("arg3={3}")
        .arg("dollarzero={0}")
        .arg("emptygroup={}")
        .arg(":::")
        .arg("a,b,c")
        .arg("d,e,f")
        .assert()
        .success()
        .stdout(predicate::eq(
            "arg1=a arg2=b arg3=c dollarzero=a,b,c emptygroup=a,b,c\narg1=d arg2=e arg3=f dollarzero=d,e,f emptygroup=d,e,f\n",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_regex_from_command_line_args_nomatch_1() {
    rust_parallel()
        .arg("-j1")
        .arg("-r")
        .arg("(.*) (.*) (.*)")
        .arg("echo")
        .arg("arg1={1}")
        .arg("arg2={2}")
        .arg("arg3={3}")
        .arg("dollarzero={0}")
        .arg(":::")
        .arg("a,b,c")
        .arg("d,e,f")
        .assert()
        .success()
        .stdout((predicate::str::contains("\n").count(2)).and(
            predicate::str::contains("regex did not match input data: a,b,c\n").and(
                predicate::str::contains("regex did not match input data: d,e,f\n"),
            ),
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn fails_invalid_regex() {
    rust_parallel()
        .arg("-r")
        .arg("((.*),(.*),(.*)")
        .arg("echo")
        .arg(":::")
        .arg("a,b,c")
        .arg("d,e,f")
        .assert()
        .failure()
        .stdout(predicate::str::contains(
            "CommandLineRegex::new: error creating regex:",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_auto_regex_from_command_line_args_j1() {
    rust_parallel()
        .arg("-j1")
        .arg("echo")
        .arg("arg1={1}")
        .arg("arg2={2}")
        .arg("dollarzero={0}")
        .arg("emptygroup={}")
        .arg(":::")
        .arg("a")
        .arg("b")
        .arg(":::")
        .arg("c")
        .arg("d")
        .assert()
        .success()
        .stdout(predicate::eq(
            "arg1=a arg2=c dollarzero=a c emptygroup=a c\narg1=a arg2=d dollarzero=a d emptygroup=a d\narg1=b arg2=c dollarzero=b c emptygroup=b c\narg1=b arg2=d dollarzero=b d emptygroup=b d\n",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_regex_from_input_file_produce_json_named_groups_j1() {
    let expected_stdout = r#"{"id": 123, "zero": "1,2,3", "empty": "1,2,3", "one": "1", "two": "2", "three": "3"}
{"id": 123, "zero": "foo,bar,baz", "empty": "foo,bar,baz", "one": "foo", "two": "bar", "three": "baz"}
"#;

    rust_parallel()
        .arg("-j1")
        .arg("-i")
        .arg("csv_file.txt")
        .arg("-r")
        .arg("(?P<arg1>.*),(?P<arg2>.*),(?P<arg3>.*)")
        .arg("echo")
        .arg(r#"{"id": 123, "zero": "{0}", "empty": "{}", "one": "{arg1}", "two": "{arg2}", "three": "{arg3}"}"#)
        .assert()
        .success()
        .stdout(predicate::eq(expected_stdout))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_regex_from_input_file_produce_json_numbered_groups_j1() {
    let expected_stdout = r#"{"id": 123, "zero": "1,2,3", "empty": "1,2,3", "three": "3", "two": "2", "one": "1"}
{"id": 123, "zero": "foo,bar,baz", "empty": "foo,bar,baz", "three": "baz", "two": "bar", "one": "foo"}
"#;

    rust_parallel()
        .arg("-j1")
        .arg("-i")
        .arg("csv_file.txt")
        .arg("-r")
        .arg("(.*),(.*),(.*)")
        .arg("echo")
        .arg(r#"{"id": 123, "zero": "{0}", "empty": "{}", "three": "{3}", "two": "{2}", "one": "{1}"}"#)
        .assert()
        .success()
        .stdout(predicate::eq(expected_stdout))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_regex_command_with_dollar_signs() {
    let expected_stdout = "input 1$ input bar\n";

    let stdin = "input";

    rust_parallel()
        .write_stdin(stdin)
        .arg("-j1")
        .arg("-r")
        .arg(".*")
        .arg("-s")
        .arg(r#"foo={0}; echo $foo 1$ "$foo" "$(echo bar)""#)
        .assert()
        .success()
        .stdout(predicate::eq(expected_stdout))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_no_run_if_empty_echo_j1() {
    let stdin = r#"

    A

    B

    C

        "#;

    rust_parallel()
        .write_stdin(stdin)
        .arg("-j1")
        .arg("--no-run-if-empty")
        .arg("echo")
        .assert()
        .success()
        .stdout(predicate::eq("A\nB\nC\n"))
        .stderr(predicate::str::is_empty());
}

#[test]
fn runs_shell_function_from_stdin_no_run_if_empty_j1() {
    let stdin = r#"

    A

    B

    C

        "#;

    rust_parallel()
        .write_stdin(stdin)
        .arg("-j1")
        .arg("-s")
        .arg("--no-run-if-empty")
        .arg("--shell-path=./dummy_shell.sh")
        .arg("shell_function")
        .assert()
        .success()
        .stdout(predicate::eq(
            "dummy_shell arg1=-c arg2=shell_function A\ndummy_shell arg1=-c arg2=shell_function B\ndummy_shell arg1=-c arg2=shell_function C\n",
        ))
        .stderr(predicate::str::is_empty());
}

#[test]
fn test_exit_status_on_failing_commands() {
    rust_parallel()
        .arg("-j1")
        .arg("cat")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .failure()
        .code(1)
        .stdout(
            (predicate::str::contains("command failed").count(3))
                .and(predicate::str::contains("command failures:"))
                .and(predicate::str::contains("exit_status_errors=3")),
        )
        .stderr(
            (predicate::str::contains("cat: A: No such file or directory").count(1))
                .and(predicate::str::contains("cat: B: No such file or directory").count(1))
                .and(predicate::str::contains("cat: C: No such file or directory").count(1)),
        );
}

#[test]
fn test_exit_status_on_failing_commands_exit_on_error() {
    rust_parallel()
        .arg("-j1")
        .arg("--exit-on-error")
        .arg("cat")
        .arg(":::")
        .arg("A")
        .arg("B")
        .arg("C")
        .assert()
        .failure()
        .code(1)
        .stdout(
            (predicate::str::contains("command failed"))
                .and(predicate::str::contains("command failures:"))
                .and(predicate::str::contains("exit_status_errors=0").not()),
        )
        .stderr(predicate::str::contains("cat: A: No such file or directory").count(1));
}
