use std::fs;
use std::path::Path;
use std::process::{Command, Output};
use tempfile::TempDir;

fn run_with_file(input: &str) -> Output {
    let directory = TempDir::new().expect("temporary directory should be created");
    let input_path = directory.path().join("transactions.csv");
    fs::write(&input_path, input).expect("input fixture should be written");
    run([input_path.as_path()])
}

fn run<'a>(arguments: impl IntoIterator<Item = &'a Path>) -> Output {
    Command::new(env!("CARGO_BIN_EXE_payments-engine"))
        .args(arguments)
        .output()
        .expect("payments engine should start")
}

#[test]
fn the_cli_writes_only_account_csv_to_stdout() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,5,900,3.7500\n",
        "withdrawal,5,901,1.2500\n",
    );

    let output = run_with_file(input);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be UTF-8"),
        concat!(
            "client,available,held,total,locked\n",
            "5,2.5000,0.0000,2.5000,false\n",
        )
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn the_cli_reports_malformed_input_on_stderr_without_partial_stdout() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,5,910,3.7500\n",
        "unknown,5,911,1.2500\n",
    );

    let output = run_with_file(input);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .expect("stderr should be UTF-8")
            .contains("invalid CSV row 3")
    );
}

#[test]
fn the_cli_reports_an_input_file_that_cannot_be_opened() {
    let directory = TempDir::new().expect("temporary directory should be created");
    let missing = directory.path().join("missing.csv");

    let output = run([missing.as_path()]);

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let diagnostic = String::from_utf8(output.stderr).expect("stderr should be UTF-8");
    assert!(diagnostic.contains("could not open"));
    assert!(diagnostic.contains("missing.csv"));
}

#[test]
fn the_cli_formats_large_equivalent_scales_exactly() {
    let input = concat!(
        "type,client,tx,amount\n",
        "deposit,5,912,7922816251426433759354395033.5000\n",
        "deposit,6,913,79228162514264337593543950335.0000\n",
    );

    let output = run_with_file(input);

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be UTF-8"),
        concat!(
            "client,available,held,total,locked\n",
            "5,7922816251426433759354395033.5000,0.0000,",
            "7922816251426433759354395033.5000,false\n",
            "6,79228162514264337593543950335.0000,0.0000,",
            "79228162514264337593543950335.0000,false\n",
        )
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn the_cli_rejects_a_missing_input_path() {
    let output = run(std::iter::empty::<&Path>());

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .expect("stderr should be UTF-8")
            .contains("usage:")
    );
}

#[test]
fn the_cli_rejects_an_extra_input_path() {
    let output = run([Path::new("first.csv"), Path::new("second.csv")]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(
        String::from_utf8(output.stderr)
            .expect("stderr should be UTF-8")
            .contains("usage:")
    );
}
