use argy::FromArgs;

#[test]
fn flatten_options() {
    #[derive(FromArgs, Debug, PartialEq)]
    /// Shared
    struct Common {
        #[argy(option)]
        /// verbose
        verbose: u32,
        #[argy(switch)]
        /// force
        force: bool,
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Parent
    struct Cmd {
        #[argy(flatten)]
        common: Common,
        #[argy(option)]
        /// output
        output: String,
    }

    let cmd =
        Cmd::from_args(&["cmd"], &["--output", "o.txt", "--verbose", "3", "--force"]).unwrap();
    assert_eq!(cmd.output, "o.txt");
    assert_eq!(cmd.common.verbose, 3);
    assert!(cmd.common.force);
}

#[test]
fn flatten_positionals_and_subcommand() {
    #[derive(FromArgs, Debug, PartialEq)]
    /// Shared
    struct Common {
        #[argy(positional)]
        /// input
        input: String,
        #[argy(subcommand)]
        sub: Option<Sub>,
    }

    #[derive(FromArgs, Debug, PartialEq)]
    #[argy(subcommand)]
    /// Sub
    enum Sub {
        /// run
        Run(RunCmd),
    }

    #[derive(FromArgs, Debug, PartialEq)]
    #[argy(subcommand, name = "run")]
    /// Run command
    struct RunCmd {
        #[argy(switch)]
        /// fast
        fast: bool,
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Parent
    struct Cmd {
        #[argy(flatten)]
        common: Common,
        #[argy(switch)]
        /// parent flag
        parent_flag: bool,
    }

    let cmd = Cmd::from_args(&["cmd"], &["in.txt", "run", "--fast"]).unwrap();
    assert_eq!(cmd.common.input, "in.txt");
    assert!(!cmd.parent_flag);
    assert!(matches!(cmd.common.sub, Some(Sub::Run(ref r)) if r.fast));
}

#[test]
#[cfg(feature = "help")]
fn flatten_help_renders_nested_options() {
    #[derive(FromArgs, Debug, PartialEq)]
    /// Shared
    struct Common {
        #[argy(option)]
        /// verbose
        verbose: u32,
        #[argy(switch)]
        /// force
        force: bool,
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Parent command
    struct Cmd {
        #[argy(flatten)]
        common: Common,
        #[argy(option)]
        /// output
        output: String,
    }

    let e = Cmd::from_args(&["cmd"], &["--help"]).expect_err("--help should exit with help");
    assert!(e.status.is_ok(), "--help status should be ok");
    assert!(e.output.contains("--verbose"), "flattened option should render: {}", e.output);
    assert!(e.output.contains("--force"), "flattened switch should render: {}", e.output);
    assert!(e.output.contains("--output"), "parent option should render: {}", e.output);
}

#[test]
fn flatten_missing_required_reports_parent_usage() {
    #[derive(FromArgs, Debug, PartialEq)]
    /// Shared
    struct Common {
        #[argy(option)]
        /// required in shared struct
        host: String,
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Parent command
    struct Cmd {
        #[argy(flatten)]
        common: Common,
        #[argy(switch)]
        /// force
        force: bool,
    }

    // A missing required option from a flattened struct is a clean usage error
    // reporting the parent command (not an unwrap panic).
    let e = Cmd::from_args(&["cmd"], &[]).expect_err("missing required option should fail");
    assert!(e.status.is_err());
    assert!(
        e.output.contains("--host"),
        "usage should name the missing flattened option: {}",
        e.output
    );
}
