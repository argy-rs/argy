// Copyright (c) 2026 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Integration tests for the process exit codes produced by `argy::from_env`
//! (via `FromEnvError::handle`).
//!
//! GNU convention: usage errors exit with status 2, while `--help`/`--version`
//! and successful parsing exit with status 0.
//!
//! `FromEnvError::handle` calls `std::process::exit`, which cannot be observed
//! in-process, so this test re-executes itself as a child process. When the
//! `ARGY_CLI_ARGS` env var is set the child acts as the CLI (parsing those args
//! and calling `handle` on failure), letting the parent observe the real exit
//! code.

use argy::FromArgs;

#[derive(FromArgs, Debug)]
#[allow(dead_code)]
/// Demo for exit-code tests.
struct ExitCodes {
    /// required value
    #[argy(option)]
    value: u32,
}

#[test]
fn exit_codes() {
    if let Ok(args) = std::env::var("ARGY_CLI_ARGS") {
        if args == "utf8" {
            // Exercise the runtime (invalid-utf8) error path directly.
            argy::FromEnvError::Utf8("bad".into()).handle();
        }
        run_cli(&args);
    }

    let exe = std::env::current_exe().unwrap();
    let run = |args: &str| {
        std::process::Command::new(&exe)
            .args(["--exact", "exit_codes", "--nocapture"])
            .env("ARGY_CLI_ARGS", args)
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
            argy::FromEnvError::EarlyExit(early_exit, "exit_codes".to_owned()).handle()
        }
    }
}

#[derive(FromArgs, Debug)]
#[allow(dead_code)]
/// Demo for requires exit-code tests.
struct RequiresExit {
    /// target host
    #[argy(option, requires = "user")]
    host: Option<String>,
    /// user name
    #[argy(option)]
    user: Option<String>,
}

#[test]
fn requires_violation_exit_code() {
    if let Ok(args) = std::env::var("ARGY_CLI_ARGS") {
        run_requires_cli(&args);
    }

    let exe = std::env::current_exe().unwrap();
    let run = |args: &str| {
        std::process::Command::new(&exe)
            .args(["--exact", "requires_violation_exit_code", "--nocapture"])
            .env("ARGY_CLI_ARGS", args)
            .output()
            .unwrap()
            .status
            .code()
    };

    // Violating requires (host without user) exits 2 (usage error).
    assert_eq!(run("--host example.com"), Some(2), "requires violation should exit 2");
    // Satisfying requires and independent options exit 0.
    assert_eq!(run("--host example.com --user alice"), Some(0));
    assert_eq!(run("--user alice"), Some(0));
}

fn run_requires_cli(input: &str) -> ! {
    let mut args: Vec<String> = vec!["requires".to_owned()];
    args.extend(input.split_whitespace().map(str::to_owned));
    let strs: Vec<&str> = args.iter().map(String::as_str).collect();

    match RequiresExit::from_args(&["requires"], &strs[1..]) {
        Ok(_) => std::process::exit(0),
        Err(early_exit) => {
            argy::FromEnvError::EarlyExit(early_exit, "requires".to_owned()).handle()
        }
    }
}
