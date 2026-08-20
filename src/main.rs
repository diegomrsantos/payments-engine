//! Command line boundary for processing one CSV file.

#![forbid(unsafe_code)]

use payments_engine::{ProcessError, process_csv};
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

/// Runs the command and maps one diagnostic to the process exit status.
fn main() -> ExitCode {
    match run(env::args_os()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            error.exit_code()
        }
    }
}

/// Processes exactly one input path and writes account CSV to locked stdout.
///
/// Stdout is passed directly to the CSV boundary, so usage and processing
/// diagnostics remain confined to stderr in [`main`].
fn run(arguments: impl IntoIterator<Item = OsString>) -> Result<(), CliError> {
    let mut arguments = arguments.into_iter();
    let program = arguments
        .next()
        .unwrap_or_else(|| OsString::from("payments-engine"));
    let input = match (arguments.next(), arguments.next()) {
        (Some(input), None) => PathBuf::from(input),
        _ => {
            return Err(CliError::Usage {
                program: program.to_string_lossy().into_owned(),
            });
        }
    };

    let file = File::open(&input).map_err(|source| CliError::Open { input, source })?;
    let stdout = io::stdout();
    process_csv(file, stdout.lock()).map_err(CliError::Process)
}

/// Failure classes that determine user diagnostics and process exit status.
#[derive(Debug)]
enum CliError {
    /// The process did not receive exactly one input path.
    Usage { program: String },
    /// The requested input path could not be opened.
    Open { input: PathBuf, source: io::Error },
    /// CSV processing failed after the input file was opened.
    Process(ProcessError),
}

impl CliError {
    /// Uses status 2 for invocation errors and status 1 for file and CSV
    /// processing failures.
    fn exit_code(&self) -> ExitCode {
        match self {
            Self::Usage { .. } => ExitCode::from(2),
            Self::Open { .. } | Self::Process(_) => ExitCode::FAILURE,
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Usage { program } => write!(formatter, "usage: {program} <transactions.csv>"),
            Self::Open { input, source } => {
                write!(formatter, "could not open {}: {source}", input.display())
            }
            Self::Process(source) => source.fmt(formatter),
        }
    }
}

impl Error for CliError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Usage { .. } => None,
            Self::Open { source, .. } => Some(source),
            Self::Process(source) => Some(source),
        }
    }
}
