#![forbid(unsafe_code)]
use codex_accounts_discovery::{encode, error_report, inspect, options, ReadError};
use std::io::{self, Write};

fn main() {
    let args: Result<Vec<String>, ReadError> = std::env::args_os()
        .skip(1)
        .map(|arg| arg.into_string().map_err(|_| ReadError::Usage))
        .collect();
    if args.as_ref().is_ok_and(|args| args == &["--help"]) {
        println!(
            "Read-only Windows registered-package discovery. Never account authority.\n\
            With no arguments, list sanitized candidate IDs.\n\
            --candidate ID             Inspect a selected opaque package ID\n\
            --runtime-relative PATH    Optional .exe inside that same package (role unqualified)\n\
            --candidate-home PATH      Optional nominated home; config.toml declaration only\n\
            --show-local-paths         LOCAL DISPLAY ONLY, not an upload-ready report\n\
            No credential files, credential stores, subprocesses or network requests.\n\
            Exit 0 means inventory/evaluation completed, not qualified. Exit 2 means refused."
        );
        return;
    }
    let result = args.and_then(options).and_then(|opts| inspect(&opts));
    let (report, failed) = match result {
        Ok(report) => (report, false),
        Err(error) => (error_report(error), true),
    };
    let (text, failed) = match encode(&report) {
        Ok(text) => (text, failed),
        Err(error) => (
            encode(&error_report(error)).expect("fixed error JSON"),
            true,
        ),
    };
    if writeln!(io::stdout().lock(), "{text}").is_err() || failed {
        std::process::exit(2);
    }
}
