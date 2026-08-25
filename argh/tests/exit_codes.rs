// Copyright (c) 2026 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Integration tests for the process exit codes produced by `argh::from_env`
//! (via `FromEnvError::handle`).
//!
//! GNU convention: usage errors exit with status 2, while `--help`/`--version`
//! and successful parsing exit with status 0.
//!
//! `FromEnvError::handle` calls `std::process::exit`, which cannot be observed
//! in-process, so this test re-executes itself as a child process. When the
//! `ARGH_CLI_ARGS` env var is set the child acts as the CLI (parsing those args
//! and calling `handle` on failure), letting the parent observe the real exit
//! code.

use argh::FromArgs;

#[derive(FromArgs, Debug)]
#[allow(dead_code)]
/// Demo for exit-code tests.
struct ExitCodes {
    /// required value
    #[argh(option)]
    value: u32,
}

#[test]
fn exit_codes() {
    if let Ok(args) = std::env::var("ARGH_CLI_ARGS") {
        if args == "utf8" {
            // Exercise the runtime (invalid-utf8) error path directly.
            argh::FromEnvError::Utf8("bad".into()).handle();
        }
        run_cli(&args);
    }

    let exe = std::env::current_exe().unwrap();
    let run = |args: &str| {
        std::process::Command::new(&exe)
            .args(["--exact", "exit_codes", "--nocapture"])
            .env("ARGH_CLI_ARGS", args)
            .output()
            .unwrap()
            .status
            .code()
    };

    // Usage errors exit with status 2.
    assert_eq!(run(""), Some(2), "missing required option should exit 2");
    assert_eq!(run("--bogus"), Some(2), "unrecognized flag should exit 2");

    // Help, version, and successful parsing exit with status 0.
    assert_eq!(run("--help"), Some(0), "--help should exit 0");
    assert_eq!(run("--version"), Some(0), "--version should exit 0");
    assert_eq!(run("--value 42"), Some(0), "successful parse should exit 0");

    // Runtime (invalid-utf8) error path still exits with status 1.
    assert_eq!(run("utf8"), Some(1), "runtime error should exit 1");
}

fn run_cli(input: &str) -> ! {
    let mut args: Vec<String> = vec!["exit_codes".to_owned()];
    args.extend(input.split_whitespace().map(str::to_owned));
    let strs: Vec<&str> = args.iter().map(String::as_str).collect();

    match ExitCodes::from_args(&["exit_codes"], &strs[1..]) {
        Ok(_) => std::process::exit(0),
        Err(early_exit) => {
            argh::FromEnvError::EarlyExit(early_exit, "exit_codes".to_owned()).handle()
        }
    }
}
