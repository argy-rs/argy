// Copyright (c) 2020 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

// Deny a bunch of uncommon clippy lints to make sure the generated code won't trigger a warning.
#![deny(
    clippy::indexing_slicing,
    clippy::panic_in_result_fn,
    clippy::str_to_string,
    clippy::unreachable,
    clippy::unwrap_in_result
)]

use {
    argy::{FromArgValue, FromArgs},
    std::fmt::Debug,
};

#[test]
fn basic_example() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Reach new heights.
    struct GoUp {
        /// whether or not to jump
        #[argy(switch, short = 'j')]
        jump: bool,

        /// how high to go
        #[argy(option)]
        height: usize,

        /// an optional nickname for the pilot
        #[argy(option)]
        pilot_nickname: Option<String>,
    }

    let up = GoUp::from_args(&["cmdname"], &["--height", "5"]).expect("failed go_up");
    assert_eq!(up, GoUp { jump: false, height: 5, pilot_nickname: None });
}

#[test]
fn option_alias() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Reach new heights.
    struct GoUp {
        /// how high to go
        #[argy(option, alias = "max-height", alias = "alt")]
        height: usize,
    }

    // The canonical name still works.
    let up = GoUp::from_args(&["cmd"], &["--height", "5"]).expect("canonical");
    assert_eq!(up, GoUp { height: 5 });

    // The aliases also work.
    let up = GoUp::from_args(&["cmd"], &["--max-height", "7"]).expect("alias 1");
    assert_eq!(up, GoUp { height: 7 });

    let up = GoUp::from_args(&["cmd"], &["--alt", "9"]).expect("alias 2");
    assert_eq!(up, GoUp { height: 9 });
}

#[test]
fn switch_alias() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Reach new heights.
    struct GoUp {
        /// whether to jump
        #[argy(switch, alias = "jmp")]
        jump: bool,
    }

    let up = GoUp::from_args(&["cmd"], &["--jmp"]).expect("alias");
    assert_eq!(up, GoUp { jump: true });
}

#[test]
fn choice_value_alias() {
    #[derive(FromArgValue, PartialEq, Debug)]
    enum Mode {
        SoftCore,
        #[argy(alias = "fast")]
        HardCore,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Do the thing.
    struct DoIt {
        #[argy(option)]
        /// how to do it
        how: Mode,
    }

    // The canonical name still works.
    let c = DoIt::from_args(&["cmd"], &["--how", "hard_core"]).expect("canonical");
    assert_eq!(c, DoIt { how: Mode::HardCore });

    // The alias also works.
    let a = DoIt::from_args(&["cmd"], &["--how", "fast"]).expect("alias");
    assert_eq!(a, DoIt { how: Mode::HardCore });
}

#[test]
fn subcommand_alias() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        #[argy(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand)]
    enum MySubCommandEnum {
        One(SubCommandOne),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// First subcommand.
    #[argy(subcommand, name = "one", alias = "uno")]
    struct SubCommandOne {
        #[argy(option)]
        /// how many x
        x: usize,
    }

    // The canonical name still works.
    let one = TopLevel::from_args(&["cmd"], &["one", "--x", "2"]).expect("canonical");
    assert_eq!(one, TopLevel { nested: MySubCommandEnum::One(SubCommandOne { x: 2 }) });

    // The alias also works.
    let uno = TopLevel::from_args(&["cmd"], &["uno", "--x", "3"]).expect("alias");
    assert_eq!(uno, TopLevel { nested: MySubCommandEnum::One(SubCommandOne { x: 3 }) });
}

#[test]
fn global_switch_accepted_before_and_after_subcommand() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        /// whether to merge
        #[argy(switch, global)]
        merge: bool,

        /// command to execute
        #[argy(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand)]
    enum MySubCommandEnum {
        Atuin(Atuin),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Import into atuin.
    #[argy(subcommand, name = "atuin")]
    struct Atuin {
        #[argy(positional)]
        name: String,
    }

    // Global switch before the subcommand.
    let before = TopLevel::from_args(&["cmd"], &["--merge", "atuin", "x"]).expect("before");
    assert_eq!(
        before,
        TopLevel { merge: true, nested: MySubCommandEnum::Atuin(Atuin { name: "x".into() }) }
    );

    // Global switch after the subcommand, before the positional.
    let after = TopLevel::from_args(&["cmd"], &["atuin", "--merge", "x"]).expect("after");
    assert_eq!(
        after,
        TopLevel { merge: true, nested: MySubCommandEnum::Atuin(Atuin { name: "x".into() }) }
    );

    // Global switch after the subcommand, after the positional.
    let after_pos = TopLevel::from_args(&["cmd"], &["atuin", "x", "--merge"]).expect("after_pos");
    assert_eq!(
        after_pos,
        TopLevel { merge: true, nested: MySubCommandEnum::Atuin(Atuin { name: "x".into() }) }
    );

    // Global switch omitted entirely.
    let none = TopLevel::from_args(&["cmd"], &["atuin", "x"]).expect("none");
    assert_eq!(
        none,
        TopLevel { merge: false, nested: MySubCommandEnum::Atuin(Atuin { name: "x".into() }) }
    );
}

#[test]
fn global_value_option_accepted_after_subcommand() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        /// verbosity level
        #[argy(option, global)]
        verbose: Option<u32>,

        /// command to execute
        #[argy(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand)]
    enum MySubCommandEnum {
        Atuin(Atuin),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Import into atuin.
    #[argy(subcommand, name = "atuin")]
    struct Atuin {}

    // Global value option before the subcommand.
    let before = TopLevel::from_args(&["cmd"], &["--verbose", "3", "atuin"]).expect("before");
    assert_eq!(before, TopLevel { verbose: Some(3), nested: MySubCommandEnum::Atuin(Atuin {}) });

    // Global value option after the subcommand.
    let after = TopLevel::from_args(&["cmd"], &["atuin", "--verbose", "4"]).expect("after");
    assert_eq!(after, TopLevel { verbose: Some(4), nested: MySubCommandEnum::Atuin(Atuin {}) });
}

#[test]
fn global_value_option_inline_after_subcommand() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        /// verbosity level
        #[argy(option, global)]
        verbose: Option<u32>,

        /// command to execute
        #[argy(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand)]
    enum MySubCommandEnum {
        Atuin(Atuin),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Import into atuin.
    #[argy(subcommand, name = "atuin")]
    struct Atuin {}

    // A global value option written with an inline `=` after the subcommand
    // must be recognized (same as the `--verbose 4` space form), not passed
    // through to the subcommand as an unrecognized argument.
    let after =
        TopLevel::from_args(&["cmd"], &["atuin", "--verbose=4"]).expect("inline after subcommand");
    assert_eq!(after, TopLevel { verbose: Some(4), nested: MySubCommandEnum::Atuin(Atuin {}) });
}

#[test]
fn non_global_option_rejected_after_subcommand() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        /// a non-global switch
        #[argy(switch)]
        local: bool,

        /// command to execute
        #[argy(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand)]
    enum MySubCommandEnum {
        Atuin(Atuin),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Import into atuin.
    #[argy(subcommand, name = "atuin")]
    struct Atuin {}

    // A non-global option is still rejected after a subcommand.
    let e = TopLevel::from_args(&["cmd"], &["atuin", "--local"]).expect_err("should reject");
    assert!(e.status.is_err());
    assert!(e.output.contains("--local"), "unexpected error output: {:?}", e.output);
}

#[test]
#[cfg(feature = "help")]
fn missing_required_argument_prints_usage() {
    #[derive(FromArgs, Debug)]
    /// Reach new heights.
    struct GoUp {
        /// how high to go
        #[argy(option)]
        _height: usize,
    }

    // A command that requires at least one argument, invoked with zero
    // arguments, must fail with a non-zero status and print the usage.
    let e = GoUp::from_args(&["cmdname"], &[]).expect_err("missing required option should fail");
    assert!(e.status.is_err());
    assert_eq!(
        e.output,
        r"Required options not provided:
    --height
Usage: cmdname --height <height>

Reach new heights.

Options:
  --height      how high to go
  --help, help  display usage information
",
    );
}

#[test]
#[cfg(feature = "help")]
fn missing_required_subcommand_prints_usage() {
    #[derive(FromArgs, Debug)]
    /// Top-level command.
    struct TopLevel {
        /// command to execute
        #[argy(subcommand)]
        _command: MySubCommandEnum,
    }

    #[derive(FromArgs, Debug)]
    #[argy(subcommand)]
    enum MySubCommandEnum {
        /// First subcommand.
        One(SubCommandOne),
    }

    #[derive(FromArgs, Debug)]
    /// First subcommand.
    #[argy(subcommand, name = "one")]
    struct SubCommandOne {}

    // A command that requires a subcommand, invoked with zero arguments, must
    // fail with a non-zero status and print the clean top-level usage — the
    // same text `--help` would produce — with no "must be present" boilerplate
    // and no full binary path.
    let e = TopLevel::from_args(&["cmdname"], &[]).expect_err("missing subcommand should fail");
    assert!(e.status.is_err());
    assert!(!e.output.contains("must be present"), "{}", e.output);
    assert!(!e.output.contains('/'), "{}", e.output);
    assert_eq!(
        e.output,
        r"Usage: cmdname <command> [<args>]

Top-level command.

Options:
  --help, help  display usage information

Commands:
  one  First subcommand.
",
    );
}

#[test]
fn generic_example() {
    use std::fmt::Display;
    use std::str::FromStr;

    #[derive(FromArgs, PartialEq, Debug)]
    /// Reach new heights.
    struct GoUp<S: FromStr>
    where
        <S as FromStr>::Err: Display,
    {
        /// whether or not to jump
        #[argy(switch, short = 'j')]
        jump: bool,

        /// how high to go
        #[argy(option)]
        height: usize,

        /// an optional nickname for the pilot
        #[argy(option)]
        pilot_nickname: Option<S>,
    }

    let up = GoUp::<String>::from_args(&["cmdname"], &["--height", "5"]).expect("failed go_up");
    assert_eq!(up, GoUp::<String> { jump: false, height: 5, pilot_nickname: None });
}

#[test]
fn custom_from_str_example() {
    #[derive(FromArgs)]
    /// Goofy thing.
    struct FiveStruct {
        /// always five
        #[argy(option, from_str_fn(always_five))]
        five: usize,
    }

    #[allow(clippy::unnecessary_wraps)] // from_str_fn requires Result
    fn always_five(_value: &str) -> Result<usize, String> {
        Ok(5)
    }

    let f = FiveStruct::from_args(&["cmdname"], &["--five", "woot"]).expect("failed to five");
    assert_eq!(f.five, 5);
}

#[test]
#[cfg(feature = "help")]
fn help_trigger_example() {
    /// Height options
    #[derive(FromArgs)]
    #[argy(help_triggers("-h", "--help", "help"))]
    struct Height {
        /// how high to go
        #[argy(option)]
        _height: usize,
    }

    assert_help_string::<Height>(
        r"Usage: test_arg_0 --height <height>

Height options

Options:
  --height          how high to go
  -h, --help, help  display usage information
",
    );
}

#[test]
fn version_trigger_example() {
    /// Height options
    #[derive(FromArgs)]
    struct Height {
        /// how high to go
        #[argy(option)]
        _height: usize,
    }

    let version = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    match Height::from_args(&["test_arg_0"], &["--version"]) {
        Ok(_) => panic!("version was parsed as args"),
        Err(e) => {
            assert_eq!(version, e.output);
            e.status.expect("version returned an error");
        }
    }
}

#[test]
fn version_trigger_custom_example() {
    /// Height options
    #[derive(FromArgs)]
    #[argy(version_triggers("-v", "--version"))]
    struct Height {
        /// how high to go
        #[argy(option)]
        _height: usize,
    }

    let version = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    for trigger in &["-v", "--version"] {
        match Height::from_args(&["test_arg_0"], &[trigger]) {
            Ok(_) => panic!("version was parsed as args"),
            Err(e) => {
                assert_eq!(version, e.output);
                e.status.expect("version returned an error");
            }
        }
    }
}

#[test]
#[cfg(feature = "help")]
fn help_render_repository_when_set() {
    #[derive(FromArgs, Debug)]
    /// Reach new heights.
    #[argy(repository)]
    struct GoUp {
        /// whether or not to jump
        #[argy(switch)]
        _jump: bool,
    }

    let output = GoUp::from_args(&["cmdname"], &["--help"])
        .expect_err("help should trigger early exit")
        .output;
    // CARGO_PKG_REPOSITORY is set for this crate, so `#[argy(repository)]` must render it.
    assert!(
        output.contains("Repository: https://github.com/argy-rs/argy"),
        "repository metadata should render when set:\n{}",
        output
    );
    assert!(
        !output.contains("Homepage:"),
        "homepage should be omitted when the attribute is absent:\n{}",
        output
    );
}

#[test]
#[cfg(feature = "help")]
fn help_omit_repository_and_homepage_when_empty() {
    #[derive(FromArgs, Debug)]
    /// Reach new heights.
    #[argy(homepage)]
    struct GoUp {
        /// whether or not to jump
        #[argy(switch)]
        _jump: bool,
    }

    let output = GoUp::from_args(&["cmdname"], &["--help"])
        .expect_err("help should trigger early exit")
        .output;
    // CARGO_PKG_HOMEPAGE is unset for this crate, so the `#[argy(homepage)]` attribute
    // must render nothing; repository is absent so it must also be omitted.
    assert!(
        !output.contains("Homepage:"),
        "homepage should be omitted when the Cargo.toml field is empty:\n{}",
        output
    );
    assert!(
        !output.contains("Repository:"),
        "repository should be omitted when the attribute is absent:\n{}",
        output
    );
}

#[test]
#[cfg(feature = "help")]
fn help_render_author_when_set() {
    #[derive(FromArgs, Debug)]
    /// Reach new heights.
    #[argy(author)]
    struct GoUp {
        /// whether or not to jump
        #[argy(switch)]
        _jump: bool,
    }

    let output = GoUp::from_args(&["cmdname"], &["--help"])
        .expect_err("help should trigger early exit")
        .output;
    // CARGO_PKG_AUTHORS is set for this crate, so `#[argy(author)]` must render it.
    assert!(output.contains("Author:"), "author metadata should render when set:\n{}", output);
    assert!(
        output.contains(env!("CARGO_PKG_AUTHORS")),
        "author metadata should contain the crate author:\n{}",
        output
    );
}

#[test]
#[cfg(feature = "help")]
fn help_omit_author_when_absent() {
    #[derive(FromArgs, Debug)]
    /// Reach new heights.
    struct GoUp {
        /// whether or not to jump
        #[argy(switch)]
        _jump: bool,
    }

    let output = GoUp::from_args(&["cmdname"], &["--help"])
        .expect_err("help should trigger early exit")
        .output;
    assert!(
        !output.contains("Author:"),
        "author should be omitted when the attribute is absent:\n{}",
        output
    );
}

#[test]
fn nested_from_str_example() {
    #[derive(FromArgs)]
    /// Goofy thing.
    struct FiveStruct {
        /// always five
        #[argy(option, from_str_fn(nested::always_five))]
        five: usize,
    }

    pub mod nested {
        #[allow(clippy::unnecessary_wraps)] // from_str_fn requires Result
        pub const fn always_five(_value: &str) -> Result<usize, String> {
            Ok(5)
        }
    }

    let f = FiveStruct::from_args(&["cmdname"], &["--five", "woot"]).expect("failed to five");
    assert_eq!(f.five, 5);
}

#[test]
fn method_from_str_example() {
    #[derive(FromArgs)]
    /// Goofy thing.
    struct FiveStruct {
        /// always five
        #[argy(option, from_str_fn(AlwaysFive::<usize>::always_five))]
        five: usize,
    }

    struct AlwaysFive<T>(T);

    impl AlwaysFive<usize> {
        #[allow(clippy::unnecessary_wraps)] // from_str_fn requires Result
        fn always_five(_value: &str) -> Result<usize, String> {
            Ok(5)
        }
    }

    let f = FiveStruct::from_args(&["cmdname"], &["--five", "woot"]).expect("failed to five");
    assert_eq!(f.five, 5);
}

#[test]
fn subcommand_example() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        #[argy(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand)]
    enum MySubCommandEnum {
        One(SubCommandOne),
        Two(Box<SubCommandTwo>),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// First subcommand.
    #[argy(subcommand, name = "one")]
    struct SubCommandOne {
        #[argy(option)]
        /// how many x
        x: usize,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Second subcommand.
    #[argy(subcommand, name = "two")]
    struct SubCommandTwo {
        #[argy(switch)]
        /// whether to fooey
        fooey: bool,
    }

    let one = TopLevel::from_args(&["cmdname"], &["one", "--x", "2"]).expect("sc 1");
    assert_eq!(one, TopLevel { nested: MySubCommandEnum::One(SubCommandOne { x: 2 }) });

    let two = TopLevel::from_args(&["cmdname"], &["two", "--fooey"]).expect("sc 2");
    assert_eq!(
        two,
        TopLevel { nested: MySubCommandEnum::Two(Box::new(SubCommandTwo { fooey: true })) }
    );
}

#[test]
#[allow(clippy::too_many_lines)] // dynamic subcommand test exercises many behaviors
fn dynamic_subcommand_example() {
    #[derive(PartialEq, Debug)]
    struct DynamicSubCommandImpl {
        got: String,
    }

    impl argy::DynamicSubCommand for DynamicSubCommandImpl {
        fn commands() -> &'static [&'static argy::CommandInfo] {
            &[
                &argy::CommandInfo {
                    name: "three",
                    short: &'\0',
                    description: "Third command",
                    aliases: &[],
                    hidden: false,
                },
                &argy::CommandInfo {
                    name: "four",
                    short: &'\0',
                    description: "Fourth command",
                    aliases: &[],
                    hidden: false,
                },
                &argy::CommandInfo {
                    name: "five",
                    short: &'\0',
                    description: "Fifth command",
                    aliases: &[],
                    hidden: false,
                },
            ]
        }

        fn try_redact_arg_values(
            _command_name: &[&str],
            _args: &[&str],
        ) -> Option<Result<Vec<String>, argy::EarlyExit>> {
            Some(Err(argy::EarlyExit::from("Test should not redact".to_owned())))
        }

        fn try_from_args(
            command_name: &[&str],
            args: &[&str],
        ) -> Option<Result<Self, argy::EarlyExit>> {
            let command_name = match command_name.last() {
                Some(x) => *x,
                None => return Some(Err(argy::EarlyExit::from("No command".to_owned()))),
            };
            let description = Self::commands().iter().find(|x| x.name == command_name)?.description;
            if args.len() > 1 {
                Some(Err(argy::EarlyExit::from("Too many arguments".to_owned())))
            } else if let Some(arg) = args.first() {
                Some(Ok(Self { got: format!("{description} got {arg:?}") }))
            } else {
                Some(Err(argy::EarlyExit::from("Not enough arguments".to_owned())))
            }
        }
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        #[argy(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand)]
    enum MySubCommandEnum {
        One(SubCommandOne),
        Two(SubCommandTwo),
        #[argy(dynamic)]
        ThreeFourFive(DynamicSubCommandImpl),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// First subcommand.
    #[argy(subcommand, name = "one")]
    struct SubCommandOne {
        #[argy(option)]
        /// how many x
        x: usize,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Second subcommand.
    #[argy(subcommand, name = "two")]
    struct SubCommandTwo {
        #[argy(switch)]
        /// whether to fooey
        fooey: bool,
    }

    let one = TopLevel::from_args(&["cmdname"], &["one", "--x", "2"]).expect("sc 1");
    assert_eq!(one, TopLevel { nested: MySubCommandEnum::One(SubCommandOne { x: 2 }) },);

    let two = TopLevel::from_args(&["cmdname"], &["two", "--fooey"]).expect("sc 2");
    assert_eq!(two, TopLevel { nested: MySubCommandEnum::Two(SubCommandTwo { fooey: true }) },);

    let three = TopLevel::from_args(&["cmdname"], &["three", "beans"]).expect("sc 3");
    assert_eq!(
        three,
        TopLevel {
            nested: MySubCommandEnum::ThreeFourFive(DynamicSubCommandImpl {
                got: "Third command got \"beans\"".to_owned()
            })
        },
    );

    let four = TopLevel::from_args(&["cmdname"], &["four", "boulders"]).expect("sc 4");
    assert_eq!(
        four,
        TopLevel {
            nested: MySubCommandEnum::ThreeFourFive(DynamicSubCommandImpl {
                got: "Fourth command got \"boulders\"".to_owned()
            })
        },
    );

    let five = TopLevel::from_args(&["cmdname"], &["five", "gold rings"]).expect("sc 5");
    assert_eq!(
        five,
        TopLevel {
            nested: MySubCommandEnum::ThreeFourFive(DynamicSubCommandImpl {
                got: "Fifth command got \"gold rings\"".to_owned()
            })
        },
    );
}

#[test]
#[cfg(feature = "help")]
fn multiline_doc_comment_description() {
    #[derive(FromArgs)]
    /// Short description
    struct Cmd {
        #[argy(switch)]
        /// a switch with a description
        /// that is spread across
        /// a number of
        /// lines of comments.
        _s: bool,
    }

    assert_help_string::<Cmd>(
        r"Usage: test_arg_0 [--s]

Short description

Options:
  --s           a switch with a description that is spread across a number of
                lines of comments.
  --help, help  display usage information
",
    );
}

#[test]
#[cfg(feature = "help")]
fn escaped_doc_comment_description() {
    #[derive(FromArgs)]
    /// A \description\:
    /// \!\"\#\$\%\&\'\(\)\*\+\,\-\.\/\:\;\<\=\>\?\@\[\\\]\^\_\`\{\|\}\~\
    struct Cmd {
        #[argy(switch)]
        /// a \description\:
        /// \!\"\#\$\%\&\'\(\)\*\+\,\-\.\/\:\;\<\=\>\?\@\[\\\]\^\_\`\{\|\}\~\
        _s: bool,
    }

    assert_help_string::<Cmd>(
        r##"Usage: test_arg_0 [--s]

A \description: !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~\

Options:
  --s           a \description: !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~\
  --help, help  display usage information
"##,
    );
}

#[test]
fn explicit_long_value_for_option() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option, long = "foo")]
        /// bar bar
        x: u8,
    }

    let cmd = Cmd::from_args(&["cmdname"], &["--foo", "5"]).unwrap();
    assert_eq!(cmd.x, 5);
}

#[test]
fn raw_identifier() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(switch)]
        /// whether to move the file
        r#move: bool,
    }

    let cmd = Cmd::from_args(&["cmdname"], &["--move"]).unwrap();
    assert!(cmd.r#move);
}

/// Test that descriptions can start with an initialism despite
/// usually being required to start with a lowercase letter.
#[derive(FromArgs)]
#[allow(unused)]
struct DescriptionStartsWithInitialism {
    /// URL fooey
    #[argy(option)]
    x: u8,
}

#[test]
fn default_number() {
    #[derive(FromArgs)]
    /// Short description
    struct Cmd {
        #[argy(option, default = "5")]
        /// fooey
        x: u8,
    }

    let cmd = Cmd::from_args(&["cmdname"], &[]).unwrap();
    assert_eq!(cmd.x, 5);
}

#[test]
fn default_function() {
    const MSG: &str = "hey I just met you";
    fn call_me_maybe() -> String {
        MSG.to_owned()
    }

    #[derive(FromArgs)]
    /// Short description
    struct Cmd {
        #[argy(option, default = "call_me_maybe()")]
        /// fooey
        msg: String,
    }

    let cmd = Cmd::from_args(&["cmdname"], &[]).unwrap();
    assert_eq!(cmd.msg, MSG);
}

#[test]
fn missing_option_value() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option)]
        /// fooey
        _msg: String,
    }

    let e = Cmd::from_args(&["cmdname"], &["--msg"])
        .expect_err("Parsing missing option value should fail");
    assert_eq!(e.output, "No value provided for option \'--msg\'.\n");
    assert!(e.status.is_err());
}

#[test]
fn env_provides_value_when_flag_absent() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option, env = "ARGY_TEST_ENV_OPT_VALUE")]
        /// fooey
        x: String,
    }

    std::env::set_var("ARGY_TEST_ENV_OPT_VALUE", "from-env");
    let cmd = Cmd::from_args(&["cmdname"], &[]).unwrap();
    std::env::remove_var("ARGY_TEST_ENV_OPT_VALUE");
    assert_eq!(cmd.x, "from-env");
}

#[test]
fn env_cli_value_takes_precedence() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option, env = "ARGY_TEST_ENV_OPT_CLI")]
        /// fooey
        x: String,
    }

    std::env::set_var("ARGY_TEST_ENV_OPT_CLI", "from-env");
    let cmd = Cmd::from_args(&["cmdname"], &["--x", "from-cli"]).unwrap();
    std::env::remove_var("ARGY_TEST_ENV_OPT_CLI");
    assert_eq!(cmd.x, "from-cli");
}

#[test]
fn env_satisfies_required_option() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option, env = "ARGY_TEST_ENV_OPT_REQUIRED")]
        /// fooey
        x: u32,
    }

    std::env::set_var("ARGY_TEST_ENV_OPT_REQUIRED", "42");
    let cmd = Cmd::from_args(&["cmdname"], &[]).unwrap();
    std::env::remove_var("ARGY_TEST_ENV_OPT_REQUIRED");
    assert_eq!(cmd.x, 42);
}

#[test]
fn requires_satisfied_by_env() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Deploy command
    struct Deploy {
        #[argy(option, requires = "token")]
        /// deployment environment
        env: Option<String>,
        #[argy(option, env = "ARGY_TEST_REQUIRES_TOKEN")]
        /// auth token
        token: Option<String>,
    }

    // `--env prod` requires `--token`; the requirement is satisfied by the
    // env-provided token, so the parse must not be rejected as missing.
    std::env::set_var("ARGY_TEST_REQUIRES_TOKEN", "secret");
    let r = Deploy::from_args(&["deploy"], &["--env", "prod"]);
    std::env::remove_var("ARGY_TEST_REQUIRES_TOKEN");
    let deploy = r.expect("env-provided token should satisfy `requires`");
    assert_eq!(deploy.env.as_deref(), Some("prod"));
    assert_eq!(deploy.token.as_deref(), Some("secret"));
}

#[test]
#[cfg(feature = "help")]
fn env_missing_required_when_neither_source_present() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option, env = "ARGY_TEST_ENV_OPT_NEITHER")]
        /// fooey
        _x: u32,
    }

    std::env::remove_var("ARGY_TEST_ENV_OPT_NEITHER");
    let e = Cmd::from_args(&["cmdname"], &[]).expect_err("should fail");
    assert!(e.status.is_err());
    assert_eq!(
        e.output,
        r"Required options not provided:
    --x
Usage: cmdname --x <x>

Short description

Options:
  --x           fooey
  --help, help  display usage information
",
    );
}

#[test]
fn env_sets_switch_true() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Short description
    struct Cmd {
        #[argy(switch, env = "ARGY_TEST_ENV_SWITCH_TRUE")]
        /// fooey
        verbose: bool,
    }

    std::env::set_var("ARGY_TEST_ENV_SWITCH_TRUE", "1");
    let cmd = Cmd::from_args(&["cmdname"], &[]).unwrap();
    std::env::remove_var("ARGY_TEST_ENV_SWITCH_TRUE");
    assert!(cmd.verbose);
}

#[test]
fn env_switch_falsy_value_leaves_unset() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Short description
    struct Cmd {
        #[argy(switch, env = "ARGY_TEST_ENV_SWITCH_FALSE")]
        /// fooey
        verbose: bool,
    }

    std::env::set_var("ARGY_TEST_ENV_SWITCH_FALSE", "0");
    let cmd = Cmd::from_args(&["cmdname"], &[]).unwrap();
    std::env::remove_var("ARGY_TEST_ENV_SWITCH_FALSE");
    assert!(!cmd.verbose);
}

#[cfg(feature = "help")]
fn assert_help_string<T: FromArgs>(help_str: &str) {
    match T::from_args(&["test_arg_0"], &["--help"]) {
        Ok(_) => panic!("help was parsed as args"),
        Err(e) => {
            assert_eq!(help_str, e.output);
            e.status.expect("help returned an error");
        }
    }
}

#[allow(clippy::needless_pass_by_value)] // generic test helper; callers pass values
fn assert_output<T: FromArgs + Debug + PartialEq>(args: &[&str], expected: T) {
    let t = T::from_args(&["cmd"], args).expect("failed to parse");
    assert_eq!(t, expected);
}

fn assert_error<T: FromArgs + Debug>(args: &[&str], err_msg: &str) {
    let e = T::from_args(&["cmd"], args).expect_err("unexpectedly succeeded parsing");
    assert_eq!(err_msg, e.output);
    e.status.expect_err("error had a positive status");
}

#[test]
#[cfg(feature = "help")]
fn help_description_column_varies_with_longest_name() {
    // With only short flag names, descriptions start well before the fixed
    // 20-column width of the previous implementation.
    #[derive(FromArgs)]
    /// Short options.
    struct ShortOpts {
        #[argy(option)]
        /// a value
        _a: usize,
    }
    assert_help_string::<ShortOpts>(
        r"Usage: test_arg_0 --a <a>

Short options.

Options:
  --a           a value
  --help, help  display usage information
",
    );

    // With a longer option name, the description column widens to match,
    // proving the column is derived from the longest name in the group.
    #[derive(FromArgs)]
    /// Long options.
    #[allow(clippy::items_after_statements)] // test defines a type mid-function
    struct LongOpts {
        #[argy(option)]
        /// a value
        _a_very_long_option_name: usize,
    }
    assert_help_string::<LongOpts>(
        r"Usage: test_arg_0 --a-very-long-option-name <a-very-long-option-name>

Long options.

Options:
  --a-very-long-option-name  a value
  --help, help               display usage information
",
    );
}

mod options {
    use super::*;

    #[derive(argy::FromArgs, Debug, PartialEq)]
    /// Woot
    struct Parsed {
        #[argy(option, short = 'n')]
        /// fooey
        n: usize,
    }

    #[test]
    fn parsed() {
        assert_output(&["-n", "5"], Parsed { n: 5 });
        assert_error::<Parsed>(
            &["-n", "x"],
            r"Error parsing option '-n' with value 'x': invalid digit found in string
",
        );
    }

    #[derive(argy::FromArgs, Debug, PartialEq)]
    /// Woot
    struct Repeating {
        #[argy(option, short = 'n')]
        /// fooey
        n: Vec<String>,
    }

    #[test]
    #[cfg(feature = "help")]
    fn repeating() {
        assert_help_string::<Repeating>(
            r"Usage: test_arg_0 [-n <n...>]

Woot

Options:
  -n, --n       fooey
  --help, help  display usage information
",
        );
    }

    #[derive(argy::FromArgs, Debug, PartialEq)]
    /// Woot
    struct WithArgName {
        #[argy(option, arg_name = "name")]
        /// fooey
        option_name: Option<String>,
    }

    #[test]
    #[cfg(feature = "help")]
    fn with_arg_name() {
        assert_help_string::<WithArgName>(
            r"Usage: test_arg_0 [--option-name <name>]

Woot

Options:
  --option-name  fooey
  --help, help   display usage information
",
        );
    }

    /// Test choices
    #[derive(FromArgs, PartialEq, Debug)]
    struct WithChoices {
        /// first choice with a default
        #[argy(option, default = "TwoChoices::Chao")]
        choice1: TwoChoices,
        /// second choice.
        #[argy(option)]
        choice2: ThreeChoices,
    }

    #[derive(FromArgValue, PartialEq, Debug)]
    enum TwoChoices {
        Hola,
        Chao,
    }

    #[derive(FromArgValue, PartialEq, Debug)]
    enum ThreeChoices {
        FirstChoice,
        #[argy(name = "に")]
        Two,
        Three,
    }

    #[test]
    fn with_choices() {
        assert_output(
            &["--choice2", "three"],
            WithChoices { choice1: TwoChoices::Chao, choice2: ThreeChoices::Three },
        );
    }

    #[test]
    fn with_choices_snake_case() {
        assert_output(
            &["--choice2", "first_choice"],
            WithChoices { choice1: TwoChoices::Chao, choice2: ThreeChoices::FirstChoice },
        );
    }

    #[test]
    fn override_default() {
        assert_output(
            &["--choice2", "first_choice", "--choice1", "hola"],
            WithChoices { choice1: TwoChoices::Hola, choice2: ThreeChoices::FirstChoice },
        );
    }

    #[test]
    fn with_name_override() {
        assert_output(
            &["--choice2", "に", "--choice1", "hola"],
            WithChoices { choice1: TwoChoices::Hola, choice2: ThreeChoices::Two },
        );
    }

    #[test]
    fn invalid_choice() {
        assert_error::<WithChoices>(
            &["--choice2", "something"],
            r#"Error parsing option '--choice2' with value 'something': expected "first_choice", "に" or "three"
"#,
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Has an optional-value option.
    struct OptionalValueString {
        /// a string that may be bare or explicit
        #[argy(option, optional_value, default_missing_value = "bare-default")]
        value: String,
    }

    #[test]
    fn optional_value_bare_uses_default_missing_value() {
        assert_output(&["--value"], OptionalValueString { value: "bare-default".into() });
    }

    #[test]
    fn optional_value_equals_syntax() {
        assert_output(&["--value=explicit"], OptionalValueString { value: "explicit".into() });
    }

    #[test]
    fn optional_value_space_separated() {
        assert_output(&["--value", "explicit"], OptionalValueString { value: "explicit".into() });
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Has an optional int option.
    struct OptionalValueInt {
        /// an int that may be bare or explicit
        #[argy(option, optional_value, default_missing_value = "42")]
        n: usize,
    }

    #[test]
    fn optional_value_int_bare_and_explicit() {
        assert_output(&["--n"], OptionalValueInt { n: 42 });
        assert_output(&["--n=5"], OptionalValueInt { n: 5 });
        assert_output(&["--n", "7"], OptionalValueInt { n: 7 });
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Has an optional `PathBuf` option.
    struct OptionalValuePath {
        /// a path that may be bare or explicit
        #[argy(option, optional_value, default_missing_value = "/tmp/foo")]
        path: std::path::PathBuf,
    }

    #[test]
    fn optional_value_path_bare_and_explicit() {
        assert_output(&["--path"], OptionalValuePath { path: "/tmp/foo".into() });
        assert_output(&["--path=/tmp/bar"], OptionalValuePath { path: "/tmp/bar".into() });
        assert_output(&["--path", "/tmp/baz"], OptionalValuePath { path: "/tmp/baz".into() });
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Has an optional `Option<String>` value.
    struct OptionalValueOption {
        /// an optional value
        #[argy(option, optional_value, default_missing_value = "bare-default")]
        value: Option<String>,
    }

    #[test]
    fn optional_value_optional_absent_is_none() {
        assert_output(&[], OptionalValueOption { value: None });
        assert_output(&["--value"], OptionalValueOption { value: Some("bare-default".into()) });
        assert_output(
            &["--value=explicit"],
            OptionalValueOption { value: Some("explicit".into()) },
        );
    }

    #[test]
    fn optional_value_with_default_falls_back_when_absent() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Has an optional-value option with a default.
        struct Defaulted {
            /// falls back to `default` when absent, `default_missing_value` when bare
            #[argy(
                option,
                default = "String::from(\"absent\")",
                optional_value,
                default_missing_value = "bare"
            )]
            value: String,
        }
        assert_output(&[], Defaulted { value: "absent".into() });
        assert_output(&["--value"], Defaulted { value: "bare".into() });
        assert_output(&["--value=explicit"], Defaulted { value: "explicit".into() });
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// A normal option requiring a value.
    struct RequiredValueFallback {
        /// a normal option
        #[argy(option)]
        value: String,
    }

    #[test]
    fn required_value_fallback_requires_a_value() {
        // A normal option without `optional_value` still requires a value.
        assert_error::<RequiredValueFallback>(
            &["--value"],
            "No value provided for option '--value'.\n",
        );
    }

    #[test]
    #[cfg(feature = "help")]
    fn optional_value_usage_marker() {
        assert_help_string::<OptionalValueString>(
            r"Usage: test_arg_0 --value[=value]

Has an optional-value option.

Options:
  --value       a string that may be bare or explicit
  --help, help  display usage information
",
        );
    }
}

mod value_delimiter {
    use super::*;

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct CommaSplit {
        #[argy(option, long = "nums", value_delimiter = ',')]
        /// fooey
        nums: Vec<usize>,
    }

    #[test]
    fn comma_split_vec() {
        assert_output(&["--nums", "1,2,3"], CommaSplit { nums: vec![1, 2, 3] });
    }

    #[test]
    fn repeated_flags_append() {
        assert_output(&["--nums", "1,2", "--nums", "3"], CommaSplit { nums: vec![1, 2, 3] });
    }

    #[test]
    fn trailing_delimiter_keeps_trailing_empty() {
        assert_output(
            &["--tags", "a,b,"],
            TagList { tags: vec!["a".into(), "b".into(), String::new()] },
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct TagList {
        #[argy(option, long = "tags", value_delimiter = ',')]
        /// fooey
        tags: Vec<String>,
    }

    #[test]
    fn empty_delimiter_keeps_empty_values() {
        assert_output(
            &["--tags", "a,,b"],
            TagList { tags: vec!["a".into(), String::new(), "b".into()] },
        );
    }

    #[test]
    fn empty_middle_delimiter_errors_for_numeric() {
        assert_error::<CommaSplit>(
            &["--nums", "1,,2"],
            r"Error parsing option '--nums' with value '1,,2': cannot parse integer from empty string
",
        );
    }
}

mod positional {
    use super::*;

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct LastRepeating {
        #[argy(positional)]
        /// fooey
        a: u32,
        #[argy(positional)]
        /// fooey
        b: Vec<String>,
    }

    #[test]
    #[cfg(feature = "help")]
    fn repeating() {
        assert_output(&["5"], LastRepeating { a: 5, b: vec![] });
        assert_output(&["5", "foo"], LastRepeating { a: 5, b: vec!["foo".into()] });
        assert_output(
            &["5", "foo", "bar"],
            LastRepeating { a: 5, b: vec!["foo".into(), "bar".into()] },
        );
        assert_help_string::<LastRepeating>(
            r"Usage: test_arg_0 [--] <a> [<b...>]

Woot

Positional Arguments:
  a  fooey
  b  fooey

Options:
  --help, help  display usage information
",
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct LastRepeatingGreedy {
        #[argy(positional)]
        /// fooey
        a: u32,
        #[argy(switch)]
        /// woo
        b: bool,
        #[argy(option)]
        /// stuff
        c: Option<String>,
        #[argy(positional, greedy)]
        /// fooey
        d: Vec<String>,
    }

    #[test]
    #[cfg(feature = "help")]
    fn positional_greedy() {
        assert_output(&["5"], LastRepeatingGreedy { a: 5, b: false, c: None, d: vec![] });
        assert_output(
            &["5", "foo"],
            LastRepeatingGreedy { a: 5, b: false, c: None, d: vec!["foo".into()] },
        );
        assert_output(
            &["5", "foo", "bar"],
            LastRepeatingGreedy { a: 5, b: false, c: None, d: vec!["foo".into(), "bar".into()] },
        );
        assert_output(
            &["5", "--b", "foo", "bar"],
            LastRepeatingGreedy { a: 5, b: true, c: None, d: vec!["foo".into(), "bar".into()] },
        );
        assert_output(
            &["5", "foo", "bar", "--b"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: None,
                d: vec!["foo".into(), "bar".into(), "--b".into()],
            },
        );
        assert_output(
            &["5", "--c", "hi", "foo", "bar"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: Some("hi".into()),
                d: vec!["foo".into(), "bar".into()],
            },
        );
        assert_output(
            &["5", "foo", "bar", "--c", "hi"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: None,
                d: vec!["foo".into(), "bar".into(), "--c".into(), "hi".into()],
            },
        );
        assert_output(
            &["5", "foo", "bar", "--", "hi"],
            LastRepeatingGreedy {
                a: 5,
                b: false,
                c: None,
                d: vec!["foo".into(), "bar".into(), "--".into(), "hi".into()],
            },
        );
        assert_help_string::<LastRepeatingGreedy>(
            r"Usage: test_arg_0 [--b] [--c <c>] [--] <a> [d...]

Woot

Positional Arguments:
  a  fooey

Options:
  --b           woo
  --c           stuff
  --help, help  display usage information
",
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct WithLast {
        #[argy(option)]
        /// stuff
        c: Option<String>,
        #[argy(switch)]
        /// woo
        b: bool,
        #[argy(positional, last)]
        /// trailing
        d: Vec<String>,
    }

    #[test]
    #[cfg(feature = "help")]
    fn last() {
        // Empty case: no trailing args.
        assert_output(&[], WithLast { c: None, b: false, d: vec![] });
        // Options before the trailing positional.
        assert_output(
            &["--c", "hi", "a", "b"],
            WithLast { c: Some("hi".into()), b: false, d: vec!["a".into(), "b".into()] },
        );
        // Options after the trailing positional are still parsed, not swallowed.
        assert_output(
            &["a", "--c", "hi", "b"],
            WithLast { c: Some("hi".into()), b: false, d: vec!["a".into(), "b".into()] },
        );
        // Mixed flags interleaved with trailing positional values.
        assert_output(
            &["a", "--b", "c", "--c", "hi", "d"],
            WithLast { c: Some("hi".into()), b: true, d: vec!["a".into(), "c".into(), "d".into()] },
        );
        // `--` separates everything after it into the trailing positional.
        assert_output(
            &["--c", "hi", "--", "a", "--b", "c"],
            WithLast {
                c: Some("hi".into()),
                b: false,
                d: vec!["a".into(), "--b".into(), "c".into()],
            },
        );
        assert_help_string::<WithLast>(
            r"Usage: test_arg_0 [--c <c>] [--b] [-- <d...>]

Woot

Positional Arguments:
  d  trailing

Options:
  --c           stuff
  --b           woo
  --help, help  display usage information
",
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct WithFirstAndLast {
        #[argy(positional)]
        /// fooey
        first: String,
        #[argy(positional, last)]
        /// trailing
        rest: Vec<String>,
    }

    #[test]
    fn last_after_plain_positional() {
        assert_output(
            &["a", "b", "c"],
            WithFirstAndLast { first: "a".into(), rest: vec!["b".into(), "c".into()] },
        );
        assert_output(&["a"], WithFirstAndLast { first: "a".into(), rest: vec![] });
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct RequiredGreedy {
        #[argy(positional)]
        /// fooey
        a: u32,
        #[argy(positional, greedy, required)]
        /// fooey
        d: Vec<String>,
    }

    #[test]
    fn positional_greedy_required() {
        // Zero values supplied: the required greedy positional must error.
        assert_error::<RequiredGreedy>(
            &["5"],
            r"Required positional arguments not provided:
    d
Usage: cmd [--] <a> d...

Woot

Positional Arguments:
  a  fooey

Options:
  --help, help  display usage information
",
        );

        // One or more values supplied: parses normally.
        assert_output(&["5", "foo"], RequiredGreedy { a: 5, d: vec!["foo".into()] });
        assert_output(
            &["5", "foo", "bar"],
            RequiredGreedy { a: 5, d: vec!["foo".into(), "bar".into()] },
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct LastOptional {
        #[argy(positional)]
        /// fooey
        a: u32,
        #[argy(positional)]
        /// fooey
        b: Option<String>,
    }

    #[test]
    fn optional() {
        assert_output(&["5"], LastOptional { a: 5, b: None });
        assert_output(&["5", "6"], LastOptional { a: 5, b: Some("6".into()) });
        assert_error::<LastOptional>(&["5", "6", "7"], "Unrecognized argument: 7\n");
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct LastDefaulted {
        #[argy(positional)]
        /// fooey
        a: u32,
        #[argy(positional, default = "5")]
        /// fooey
        b: u32,
    }

    #[test]
    fn defaulted() {
        assert_output(&["5"], LastDefaulted { a: 5, b: 5 });
        assert_output(&["5", "6"], LastDefaulted { a: 5, b: 6 });
        assert_error::<LastDefaulted>(&["5", "6", "7"], "Unrecognized argument: 7\n");
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct LastRequired {
        #[argy(positional)]
        /// fooey
        a: u32,
        #[argy(positional)]
        /// fooey
        b: u32,
    }

    #[test]
    fn required() {
        assert_output(&["5", "6"], LastRequired { a: 5, b: 6 });
        assert_error::<LastRequired>(
            &[],
            r"Required positional arguments not provided:
    a
    b
Usage: cmd [--] <a> <b>

Woot

Positional Arguments:
  a  fooey
  b  fooey

Options:
  --help, help  display usage information
",
        );
        assert_error::<LastRequired>(
            &["5"],
            r"Required positional arguments not provided:
    b
Usage: cmd [--] <a> <b>

Woot

Positional Arguments:
  a  fooey
  b  fooey

Options:
  --help, help  display usage information
",
        );
    }

    #[derive(argy::FromArgs, Debug, PartialEq)]
    /// Woot
    struct Parsed {
        #[argy(positional)]
        /// fooey
        n: usize,
    }

    #[test]
    fn parsed() {
        assert_output(&["5"], Parsed { n: 5 });
        assert_error::<Parsed>(
            &["x"],
            r"Error parsing positional argument 'n' with value 'x': invalid digit found in string
",
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct WithOption {
        #[argy(positional)]
        /// fooey
        a: String,
        #[argy(option)]
        /// fooey
        b: String,
    }

    #[test]
    fn mixed_with_option() {
        assert_output(&["first", "--b", "foo"], WithOption { a: "first".into(), b: "foo".into() });

        assert_error::<WithOption>(
            &[],
            r"Required positional arguments not provided:
    a
Required options not provided:
    --b
Usage: cmd --b <b> [--] <a>

Woot

Positional Arguments:
  a  fooey

Options:
  --b           fooey
  --help, help  display usage information
",
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct WithSubcommand {
        #[argy(positional)]
        /// fooey
        a: String,
        #[argy(subcommand)]
        /// fooey
        b: Subcommand,
        #[argy(positional)]
        /// fooey
        c: Vec<String>,
    }

    #[derive(FromArgs, Debug, PartialEq)]
    #[argy(subcommand, name = "a")]
    #[allow(clippy::doc_markdown)] // doc text is rendered verbatim in help output
    /// Subcommand of positional::WithSubcommand.
    struct Subcommand {
        #[argy(positional)]
        /// fooey
        a: String,
        #[argy(positional)]
        /// fooey
        b: Vec<String>,
    }

    #[test]
    fn mixed_with_subcommand() {
        assert_output(
            &["first", "a", "a"],
            WithSubcommand {
                a: "first".into(),
                b: Subcommand { a: "a".into(), b: vec![] },
                c: vec![],
            },
        );

        assert_error::<WithSubcommand>(
            &["a", "a", "a"],
            r"Required positional arguments not provided:
    a
Usage: cmd <a> [<c...>] <command> [<args>]

Woot

Positional Arguments:
  a  fooey
  c  fooey

Options:
  --help, help  display usage information

Commands:
  a  Subcommand of positional::WithSubcommand.
",
        );

        assert_output(
            &["1", "2", "3", "a", "b", "c"],
            WithSubcommand {
                a: "1".into(),
                b: Subcommand { a: "b".into(), b: vec!["c".into()] },
                c: vec!["2".into(), "3".into()],
            },
        );
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Woot
    struct Underscores {
        #[argy(positional)]
        /// fooey
        a_: String,
    }

    #[test]
    fn positional_name_with_underscores() {
        assert_output(&["first"], Underscores { a_: "first".into() });

        assert_error::<Underscores>(
            &[],
            r"Required positional arguments not provided:
    a
Usage: cmd [--] <a>

Woot

Positional Arguments:
  a  fooey

Options:
  --help, help  display usage information
",
        );
    }
}

/// Tests derived from
/// <https://fuchsia.dev/fuchsia-src/development/api/cli> and
/// <https://fuchsia.dev/fuchsia-src/development/api/cli_help>
mod fuchsia_commandline_tools_rubric {
    use super::*;

    /// Tests for the three required command line argument types:
    /// - exact text
    /// - arguments
    /// - options (i.e. switches and keys)
    #[test]
    fn three_command_line_argument_types() {
        // TODO(cramertj) add support for exact text and positional arguments
    }

    /// A piece of exact text may be required or optional
    #[test]
    fn exact_text_required_and_optional() {
        // TODO(cramertj) add support for exact text
    }

    /// Arguments are like function parameters or slots for data.
    /// The order often matters.
    #[test]
    fn arguments_ordered() {
        // TODO(cramertj) add support for ordered positional arguments
    }

    /// If a single argument is repeated, order may not matter, e.g. `<files>...`
    #[test]
    fn arguments_unordered() {
        // TODO(cramertj) add support for repeated positional arguments
    }

    // Short argument names must use one dash and a single letter.
    // TODO(cramertj): this should be a compile-fail test

    // Short argument names are optional, but all choices are required to have a `--` option.
    // TODO(cramertj): this should be a compile-fail test

    // Numeric options, such as `-1` and `-2`, are not allowed.
    // TODO(cramertj): this should be a compile-fail test

    #[derive(FromArgs)]
    /// One switch.
    struct OneSwitch {
        #[argy(switch, short = 's')]
        /// just a switch
        switchy: bool,
    }

    /// The presence of a switch means the feature it represents is "on",
    /// while its absence means that it is "off".
    #[test]
    fn switch_on_when_present() {
        let on = OneSwitch::from_args(&["cmdname"], &["-s"]).expect("parsing on");
        assert!(on.switchy);

        let off = OneSwitch::from_args(&["cmdname"], &[]).expect("parsing off");
        assert!(!off.switchy);
    }

    #[derive(FromArgs, Debug)]
    /// Two Switches
    struct TwoSwitches {
        #[argy(switch, short = 'a')]
        /// a
        a: bool,
        #[argy(switch, short = 'b')]
        /// b
        b: bool,
    }

    /// Switches may be clustered: `-ab` behaves like `-a -b`.
    #[test]
    fn switches_can_be_clustered() {
        let clustered = TwoSwitches::from_args(&["cmdname"], &["-ab"]).expect("clustered -ab");
        let separate = TwoSwitches::from_args(&["cmdname"], &["-a", "-b"]).expect("separate -a -b");
        assert!(clustered.a && clustered.b);
        assert_eq!(clustered.a, separate.a);
        assert_eq!(clustered.b, separate.b);
    }

    #[derive(FromArgs, Debug)]
    /// Three Switches
    struct ThreeSwitches {
        #[argy(switch, short = 'a')]
        /// a
        a: bool,
        #[argy(switch, short = 'b')]
        /// b
        b: bool,
        #[argy(switch, short = 'c')]
        /// c
        c: bool,
    }

    /// A cluster of three switches behaves identically to the same switches
    /// given separately.
    #[test]
    fn three_switches_can_be_clustered() {
        let clustered = ThreeSwitches::from_args(&["cmdname"], &["-abc"]).expect("clustered -abc");
        let separate =
            ThreeSwitches::from_args(&["cmdname"], &["-a", "-b", "-c"]).expect("separate -a -b -c");
        assert!(clustered.a && clustered.b && clustered.c);
        assert_eq!(clustered.a, separate.a);
        assert_eq!(clustered.b, separate.b);
        assert_eq!(clustered.c, separate.c);
    }

    #[derive(FromArgs, Debug)]
    /// A switch followed by a value-taking option
    struct SwitchThenOption {
        #[argy(switch, short = 'v')]
        /// verbose
        verbose: bool,
        #[argy(option, short = 'n')]
        /// count
        count: usize,
    }

    /// A cluster ending in a value-taking short consumes the remainder of the
    /// cluster as that short's value, and falls back to the next argument when
    /// the cluster ends at the value-taking short.
    #[test]
    fn cluster_ending_in_value_taking_short_consumes_remainder() {
        let clustered =
            SwitchThenOption::from_args(&["cmdname"], &["-vn5"]).expect("clustered -vn5");
        assert!(clustered.verbose);
        assert_eq!(clustered.count, 5);

        let next_arg =
            SwitchThenOption::from_args(&["cmdname"], &["-vn", "7"]).expect("-vn with next arg");
        assert!(next_arg.verbose);
        assert_eq!(next_arg.count, 7);
    }

    #[derive(FromArgs, Debug)]
    /// One keyed option
    struct OneOption {
        #[argy(option)]
        /// some description
        _foo: String,
    }

    /// `--opt=value` and `--opt value` are equivalent for long options.
    #[test]
    #[allow(clippy::used_underscore_binding)] // `_foo` field keeps the `--foo` option name
    fn long_option_equals_is_equivalent_to_space_separated() {
        let with_equals = OneOption::from_args(&["cmdname"], &["--foo=bar"])
            .expect("Parsing option value using `=` should succeed");
        let with_space = OneOption::from_args(&["cmdname"], &["--foo", "bar"])
            .expect("Parsing option value as separate arg should succeed");
        assert_eq!(with_equals._foo, "bar");
        assert_eq!(with_equals._foo, with_space._foo);
    }

    /// `--opt=` with an empty inline value yields an empty string value.
    #[test]
    #[allow(clippy::used_underscore_binding)] // `_foo` field keeps the `--foo` option name
    fn long_option_equals_with_empty_value() {
        let parsed = OneOption::from_args(&["cmdname"], &["--foo="])
            .expect("Parsing an empty inline value should succeed");
        assert_eq!(parsed._foo, "");
    }

    /// A switch (flag) does not take a value, so `--flag=value` is an error.
    #[test]
    fn switch_rejects_inline_value() {
        let Err(e) = OneSwitch::from_args(&["cmdname"], &["--switchy=true"]) else {
            panic!("Parsing a switch with an inline value should fail")
        };
        assert_eq!(
            e.output,
            "Error parsing option '--switchy' with value 'true': does not take a value\n"
        );
        assert!(e.status.is_err());
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// One optional-value switch.
    struct OptionalSwitch {
        #[argy(switch)]
        /// an optional boolean switch
        flag: Option<bool>,
    }

    /// An `Option<bool>` switch is `None` when absent.
    #[test]
    fn optional_switch_absent_is_none() {
        let parsed = OptionalSwitch::from_args(&["cmdname"], &[]).expect("parsing absent");
        assert_eq!(parsed, OptionalSwitch { flag: None });
    }

    /// A bare `--flag` sets an `Option<bool>` switch to `Some(true)`.
    #[test]
    fn optional_switch_bare_is_some_true() {
        let parsed =
            OptionalSwitch::from_args(&["cmdname"], &["--flag"]).expect("parsing bare switch");
        assert_eq!(parsed, OptionalSwitch { flag: Some(true) });
    }

    /// `--flag=true` sets an `Option<bool>` switch to `Some(true)`.
    #[test]
    fn optional_switch_true() {
        let parsed = OptionalSwitch::from_args(&["cmdname"], &["--flag=true"])
            .expect("parsing `--flag=true`");
        assert_eq!(parsed, OptionalSwitch { flag: Some(true) });
    }

    /// `--flag=false` sets an `Option<bool>` switch to `Some(false)`.
    #[test]
    fn optional_switch_false() {
        let parsed = OptionalSwitch::from_args(&["cmdname"], &["--flag=false"])
            .expect("parsing `--flag=false`");
        assert_eq!(parsed, OptionalSwitch { flag: Some(false) });
    }

    /// An invalid inline value on an `Option<bool>` switch is an error.
    #[test]
    fn optional_switch_rejects_invalid_bool_value() {
        let Err(e) = OptionalSwitch::from_args(&["cmdname"], &["--flag=xyz"]) else {
            panic!("Parsing an invalid boolean value should fail")
        };
        assert_eq!(
            e.output,
            "Error parsing option '--flag' with value 'xyz': invalid boolean value 'xyz'\n"
        );
        assert!(e.status.is_err());
    }

    // Two dashes on their own indicates the end of options.
    // Subsequent values are given to the tool as-is.
    //
    // It's unclear exactly what "are given to the tool as-is" in means in this
    // context, so we provide a few options for handling `--`, with it being
    // an error by default.
    //
    // TODO(cramertj) implement some behavior for `--`

    /// Double-dash is treated as an error by default.
    #[test]
    fn double_dash_default_error() {}

    /// Double-dash can be ignored for later manual parsing.
    #[test]
    fn double_dash_ignore() {}

    /// Double-dash should be treated as the end of flags and optional arguments,
    /// and the remainder of the values should be treated purely as positional arguments,
    /// even when their syntax matches that of options. e.g. `foo -- -e` should be parsed
    /// as passing a single positional argument with the value `-e`.
    #[test]
    fn double_dash_positional() {
        #[derive(FromArgs, Debug, PartialEq)]
        /// Positional arguments list
        struct StringList {
            #[argy(positional)]
            /// a list of strings
            strs: Vec<String>,

            #[argy(switch)]
            /// some flag
            flag: bool,
        }

        assert_output(
            &["--", "a", "-b", "--flag"],
            StringList { strs: vec!["a".into(), "-b".into(), "--flag".into()], flag: false },
        );
        assert_output(
            &["--flag", "--", "-a", "b"],
            StringList { strs: vec!["-a".into(), "b".into()], flag: true },
        );
        assert_output(&["--", "--help"], StringList { strs: vec!["--help".into()], flag: false });
        assert_output(
            &["--", "-a", "--help"],
            StringList { strs: vec!["-a".into(), "--help".into()], flag: false },
        );
    }

    /// Double-dash can be parsed into an optional field using a provided
    /// `fn(&[&str]) -> Result<T, EarlyExit>`.
    #[test]
    fn double_dash_custom() {}

    /// Repeating switches may be used to apply more emphasis.
    /// A common example is increasing verbosity by passing more `-v` switches.
    #[test]
    fn switches_repeating() {
        #[derive(FromArgs, Debug)]
        /// A type for testing repeating `-v`
        struct CountVerbose {
            #[argy(switch, short = 'v')]
            /// increase the verbosity of the command.
            verbose: i128,
        }

        let cv = CountVerbose::from_args(&["cmdname"], &["-v", "-v", "-v"])
            .expect("Parsing verbose flags should succeed");
        assert_eq!(cv.verbose, 3);
    }

    // When a tool has many subcommands, it should also have a help subcommand
    // that displays help about the subcommands, e.g. `fx help build`.
    //
    // Elsewhere in the docs, it says the syntax `--help` is required, so we
    // interpret that to mean:
    //
    // - `help` should always be accepted as a "keyword" in place of the first
    //   positional argument for both the main command and subcommands.
    //
    // - If followed by the name of a subcommand it should forward to the
    //   `--help` of said subcommand, otherwise it will fall back to the
    //   help of the righmost command / subcommand.
    //
    // - `--help` will always consider itself the only meaningful argument to
    //   the rightmost command / subcommand, and any following arguments will
    //   be treated as an error.

    #[derive(FromArgs, Debug)]
    /// A type for testing `--help`/`help`
    struct HelpTopLevel {
        #[argy(subcommand)]
        _sub: HelpFirstSub,
    }

    #[derive(FromArgs, Debug)]
    #[argy(subcommand, name = "first")]
    /// First subcommmand for testing `help`.
    struct HelpFirstSub {
        #[argy(subcommand)]
        _sub: HelpSecondSub,
    }

    #[derive(FromArgs, Debug)]
    #[argy(subcommand, name = "second")]
    /// Second subcommand for testing `help`.
    struct HelpSecondSub {}

    #[cfg(feature = "help")]
    fn expect_help(args: &[&str], expected_help_string: &str) {
        let e = HelpTopLevel::from_args(&["cmdname"], args).expect_err("should exit early");
        assert_eq!(expected_help_string, e.output);
        e.status.expect("help returned an error");
    }

    #[cfg(feature = "help")]
    const MAIN_HELP_STRING: &str = r"Usage: cmdname <command> [<args>]

A type for testing `--help`/`help`

Options:
  --help, help  display usage information

Commands:
  first  First subcommmand for testing `help`.
";

    #[cfg(feature = "help")]
    const FIRST_HELP_STRING: &str = r"Usage: cmdname first <command> [<args>]

First subcommmand for testing `help`.

Options:
  --help, help  display usage information

Commands:
  second  Second subcommand for testing `help`.
";

    #[cfg(feature = "help")]
    const SECOND_HELP_STRING: &str = r"Usage: cmdname first second

Second subcommand for testing `help`.

Options:
  --help, help  display usage information
";

    #[test]
    #[cfg(feature = "help")]
    fn help_keyword_main() {
        expect_help(&["help"], MAIN_HELP_STRING);
    }

    #[test]
    #[cfg(feature = "help")]
    fn help_keyword_with_following_subcommand() {
        expect_help(&["help", "first"], FIRST_HELP_STRING);
    }

    #[test]
    #[cfg(feature = "help")]
    fn help_keyword_between_subcommands() {
        expect_help(&["first", "help", "second"], SECOND_HELP_STRING);
    }

    #[test]
    #[cfg(feature = "help")]
    fn help_keyword_with_two_trailing_subcommands() {
        expect_help(&["help", "first", "second"], SECOND_HELP_STRING);
    }

    #[test]
    #[cfg(feature = "help")]
    fn help_flag_main() {
        expect_help(&["--help"], MAIN_HELP_STRING);
    }

    #[test]
    #[cfg(feature = "help")]
    fn help_flag_subcommand() {
        expect_help(&["first", "--help"], FIRST_HELP_STRING);
    }

    #[test]
    #[cfg(feature = "help")]
    fn help_short_flag_subcommand() {
        expect_help(&["first", "-h"], FIRST_HELP_STRING);
    }

    #[test]
    #[cfg(feature = "help")]
    fn help_long_forms_still_work_on_subcommand() {
        expect_help(&["first", "--help"], FIRST_HELP_STRING);
        expect_help(&["first", "help"], FIRST_HELP_STRING);
    }

    #[test]
    fn version_short_flag_subcommand() {
        let version =
            format!("{}-{} {}", env!("CARGO_PKG_NAME"), "first", env!("CARGO_PKG_VERSION"));
        for trigger in &["-V", "--version"] {
            match HelpTopLevel::from_args(&["cmdname"], &["first", trigger]) {
                Ok(_) => panic!("version was parsed as args"),
                Err(e) => {
                    assert_eq!(version, e.output);
                    e.status.expect("version returned an error");
                }
            }
        }
    }

    #[test]
    fn version_output_is_qualified_for_subcommands_but_not_top_level() {
        let top_level = format!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        let subcommand =
            format!("{}-{} {}", env!("CARGO_PKG_NAME"), "first", env!("CARGO_PKG_VERSION"));

        let top = HelpTopLevel::from_args(&["cmdname"], &["--version"])
            .expect_err("version should exit early");
        assert_eq!(top_level, top.output);

        let sub = HelpTopLevel::from_args(&["cmdname"], &["first", "--version"])
            .expect_err("version should exit early");
        assert_eq!(subcommand, sub.output);
    }

    #[test]
    fn help_flag_trailing_arguments_are_an_error() {
        let e = OneOption::from_args(&["cmdname"], &["--help", "--foo", "bar"])
            .expect_err("should exit early");
        assert_eq!("Trailing arguments are not allowed after `help`.", e.output);
        e.status.expect_err("should be an error");
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(
        description = "Destroy the contents of <file>.",
        example = "Scribble 'abc' and then run |grind|.\n$ {command_name} -s 'abc' grind old.txt taxes.cp",
        note = "Use `{command_name} help <command>` for details on [<args>] for a subcommand.",
        error_code(2, "The blade is too dull."),
        error_code(3, "Out of fuel.")
    )]
    struct HelpExample {
        /// force, ignore minor errors. This description is so long that it wraps to the next line.
        #[argy(switch, short = 'f')]
        force: bool,

        /// documentation
        #[argy(switch)]
        really_really_really_long_name_for_pat: bool,

        /// write <scribble> repeatedly
        #[argy(option, short = 's')]
        scribble: String,

        #[allow(clippy::doc_markdown)] // doc text is rendered verbatim in help output
        /// say more. Defaults to $BLAST_VERBOSE.
        #[argy(switch, short = 'v')]
        verbose: bool,

        #[argy(subcommand)]
        command: HelpExampleSubCommands,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand)]
    enum HelpExampleSubCommands {
        BlowUp(BlowUp),
        Grind(GrindCommand),
        #[argy(dynamic)]
        Plugin(HelpExamplePlugin),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand, name = "blow-up")]
    /// explosively separate
    struct BlowUp {
        /// blow up bombs safely
        #[argy(switch)]
        safely: bool,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand, name = "grind", description = "make smaller by many small cuts")]
    struct GrindCommand {
        /// wear a visor while grinding
        #[argy(switch)]
        safely: bool,
    }

    #[derive(PartialEq, Debug)]
    struct HelpExamplePlugin {
        got: String,
    }

    impl argy::DynamicSubCommand for HelpExamplePlugin {
        fn commands() -> &'static [&'static argy::CommandInfo] {
            &[&argy::CommandInfo {
                name: "plugin",
                short: &'\0',
                description: "Example dynamic command",
                aliases: &[],
                hidden: false,
            }]
        }

        fn try_redact_arg_values(
            _command_name: &[&str],
            _args: &[&str],
        ) -> Option<Result<Vec<String>, argy::EarlyExit>> {
            Some(Err(argy::EarlyExit::from("Test should not redact".to_owned())))
        }

        fn try_from_args(
            command_name: &[&str],
            args: &[&str],
        ) -> Option<Result<Self, argy::EarlyExit>> {
            if command_name.last() != Some(&"plugin") {
                None
            } else if args.len() > 1 {
                Some(Err(argy::EarlyExit::from("Too many arguments".to_owned())))
            } else if let Some(arg) = args.first() {
                Some(Ok(Self { got: format!("plugin got {arg:?}") }))
            } else {
                Some(Ok(Self { got: "plugin got no argument".to_owned() }))
            }
        }
    }

    #[test]
    fn example_parses_correctly() {
        let help_example = HelpExample::from_args(
            &["program-name"],
            &["-f", "--scribble", "fooey", "blow-up", "--safely"],
        )
        .unwrap();

        assert_eq!(
            help_example,
            HelpExample {
                force: true,
                scribble: "fooey".to_owned(),
                really_really_really_long_name_for_pat: false,
                verbose: false,
                command: HelpExampleSubCommands::BlowUp(BlowUp { safely: true }),
            },
        );
    }

    #[test]
    fn example_errors_on_missing_required_option_and_missing_required_subcommand() {
        let exit = HelpExample::from_args(&["program-name"], &[]).unwrap_err();
        exit.status.unwrap_err();
        assert_eq!(
            exit.output,
            r"Required options not provided:
    --scribble
Usage: program-name [-f] [--really-really-really-long-name-for-pat] -s <scribble> [-v] <command> [<args>]

Destroy the contents of <file>.

Options:
  -f, --force                               force, ignore minor errors. This
                                            description is so long that it wraps
                                            to the next line.
  --really-really-really-long-name-for-pat  documentation
  -s, --scribble                            write <scribble> repeatedly
  -v, --verbose                             say more. Defaults to
                                            $BLAST_VERBOSE.
  --help, help                              display usage information

Commands:
  blow-up  explosively separate
  grind    make smaller by many small cuts
  plugin   Example dynamic command

Examples:
  Scribble 'abc' and then run |grind|.
  $ program-name -s 'abc' grind old.txt taxes.cp

Notes:
  Use `program-name help <command>` for details on [<args>] for a subcommand.

Error codes:
  2 The blade is too dull.
  3 Out of fuel.
",
        );
    }

    #[test]
    #[cfg(feature = "help")]
    fn help_example() {
        assert_help_string::<HelpExample>(
            r"Usage: test_arg_0 [-f] [--really-really-really-long-name-for-pat] -s <scribble> [-v] <command> [<args>]

Destroy the contents of <file>.

Options:
  -f, --force                               force, ignore minor errors. This
                                            description is so long that it wraps
                                            to the next line.
  --really-really-really-long-name-for-pat  documentation
  -s, --scribble                            write <scribble> repeatedly
  -v, --verbose                             say more. Defaults to
                                            $BLAST_VERBOSE.
  --help, help                              display usage information

Commands:
  blow-up  explosively separate
  grind    make smaller by many small cuts
  plugin   Example dynamic command

Examples:
  Scribble 'abc' and then run |grind|.
  $ test_arg_0 -s 'abc' grind old.txt taxes.cp

Notes:
  Use `test_arg_0 help <command>` for details on [<args>] for a subcommand.

Error codes:
  2 The blade is too dull.
  3 Out of fuel.
",
        );
    }

    #[allow(dead_code)]
    #[derive(argy::FromArgs)]
    /// Destroy the contents of <file>.
    struct WithArgName {
        #[argy(positional, arg_name = "name")]
        username: String,
    }

    #[test]
    #[cfg(feature = "help")]
    fn with_arg_name() {
        assert_help_string::<WithArgName>(
            r"Usage: test_arg_0 [--] <name>

Destroy the contents of <file>.

Positional Arguments:
  name

Options:
  --help, help  display usage information
",
        );
    }

    #[test]
    #[cfg(feature = "help")]
    fn hidden_help_attribute() {
        #[derive(FromArgs)]
        /// Short description
        struct Cmd {
            /// this one should be hidden
            #[argy(positional, hidden_help)]
            _one: String,
            #[argy(positional)]
            /// this one is real
            _two: String,
            /// this one should be hidden
            #[argy(option, hidden_help)]
            _three: String,
        }

        assert_help_string::<Cmd>(
            r"Usage: test_arg_0 [--] <two>

Short description

Positional Arguments:
  two  this one is real

Options:
  --help, help  display usage information
",
        );
    }
}

#[test]
#[cfg(feature = "help")]
fn hidden_subcommand_omitted_from_help_but_still_runs() {
    #[derive(FromArgs, Debug, PartialEq)]
    /// Top-level command.
    struct TopLevel {
        #[argy(subcommand)]
        cmd: Subcommands,
    }

    #[derive(FromArgs, Debug, PartialEq)]
    #[argy(subcommand)]
    enum Subcommands {
        Visible(Visible),
        Secret(Secret),
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Visible subcommand.
    #[argy(subcommand, name = "visible")]
    struct Visible {
        #[argy(switch)]
        /// a flag
        foo: bool,
    }

    #[derive(FromArgs, Debug, PartialEq)]
    /// Secret internal subcommand.
    #[argy(subcommand, name = "secret", hidden)]
    struct Secret {
        #[argy(switch)]
        /// a flag
        bar: bool,
    }

    // Help lists the visible subcommand but omits the hidden one.
    let help = TopLevel::from_args(&["cmd"], &["--help"]).unwrap_err().output;
    assert!(help.contains("visible"), "visible subcommand should be listed in help");
    assert!(!help.contains("secret"), "hidden subcommand should be omitted from help");

    // The hidden subcommand is still invocable.
    let parsed = TopLevel::from_args(&["cmd"], &["secret", "--bar"]).unwrap();
    assert_eq!(parsed, TopLevel { cmd: Subcommands::Secret(Secret { bar: true }) });
}

#[test]
fn redact_arg_values_no_args() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option)]
        /// a msg param
        _msg: Option<String>,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &[]).unwrap();
    assert_eq!(actual, &["program-name"]);
}

#[test]
fn redact_arg_values_optional_arg() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option)]
        /// a msg param
        _msg: Option<String>,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &["--msg", "hello"]).unwrap();
    assert_eq!(actual, &["program-name", "--msg"]);
}

#[test]
fn redact_arg_values_optional_arg_short() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option, short = 'm')]
        /// a msg param
        _msg: Option<String>,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &["-m", "hello"]).unwrap();
    assert_eq!(actual, &["program-name", "-m"]);
}

#[test]
fn redact_arg_values_optional_arg_long() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option, long = "my-msg")]
        /// a msg param
        _msg: Option<String>,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &["--my-msg", "hello"]).unwrap();
    assert_eq!(actual, &["program-name", "--my-msg"]);
}

#[test]
fn redact_arg_values_two_option_args() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option)]
        /// a msg param
        _msg: String,

        #[argy(option)]
        /// a delivery param
        _delivery: String,
    }

    let actual =
        Cmd::redact_arg_values(&["program-name"], &["--msg", "hello", "--delivery", "next day"])
            .unwrap();
    assert_eq!(actual, &["program-name", "--msg", "--delivery"]);
}

#[test]
fn redact_arg_values_option_one_optional_args() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option)]
        /// a msg param
        _msg: String,

        #[argy(option)]
        /// a delivery param
        _delivery: Option<String>,
    }

    let actual =
        Cmd::redact_arg_values(&["program-name"], &["--msg", "hello", "--delivery", "next day"])
            .unwrap();
    assert_eq!(actual, &["program-name", "--msg", "--delivery"]);

    let actual = Cmd::redact_arg_values(&["program-name"], &["--msg", "hello"]).unwrap();
    assert_eq!(actual, &["program-name", "--msg"]);
}

#[test]
fn redact_arg_values_option_repeating() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(option)]
        /// fooey
        _msg: Vec<String>,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &[]).unwrap();
    assert_eq!(actual, &["program-name"]);

    let actual =
        Cmd::redact_arg_values(&["program-name"], &["--msg", "abc", "--msg", "xyz"]).unwrap();
    assert_eq!(actual, &["program-name", "--msg", "--msg"]);
}

#[test]
fn redact_arg_values_switch() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(switch, short = 'f')]
        /// speed of cmd
        _faster: bool,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &["--faster"]).unwrap();
    assert_eq!(actual, &["program-name", "--faster"]);

    let actual = Cmd::redact_arg_values(&["program-name"], &["-f"]).unwrap();
    assert_eq!(actual, &["program-name", "-f"]);
}

#[test]
fn redact_arg_values_positional() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[allow(unused)]
        #[argy(positional)]
        /// speed of cmd
        speed: u8,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &["5"]).unwrap();
    assert_eq!(actual, &["program-name", "speed"]);
}

#[test]
fn redact_arg_values_positional_arg_name() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(positional, arg_name = "speed")]
        /// speed of cmd
        _speed: u8,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &["5"]).unwrap();
    assert_eq!(actual, &["program-name", "speed"]);
}

#[test]
fn redact_arg_values_positional_repeating() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(positional, arg_name = "speed")]
        /// speed of cmd
        _speed: Vec<u8>,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &["5", "6"]).unwrap();
    assert_eq!(actual, &["program-name", "speed", "speed"]);
}

#[test]
fn redact_arg_values_positional_err() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(positional, arg_name = "speed")]
        /// speed of cmd
        _speed: u8,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &[]).unwrap_err();
    assert_eq!(
        actual,
        argy::EarlyExit {
            output: "Required positional arguments not provided:\n    speed\nUsage: program-name [--] <speed>\n\nShort description\n\nPositional Arguments:\n  speed  speed of cmd\n\nOptions:\n  --help, help  display usage information\n"
                .into(),
            status: Err(()),
        }
    );
}

#[test]
fn redact_arg_values_two_positional() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(positional, arg_name = "speed")]
        /// speed of cmd
        _speed: u8,

        #[argy(positional, arg_name = "direction")]
        /// direction
        _direction: String,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &["5", "north"]).unwrap();
    assert_eq!(actual, &["program-name", "speed", "direction"]);
}

#[test]
fn redact_arg_values_positional_option() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(positional, arg_name = "speed")]
        /// speed of cmd
        _speed: u8,

        #[argy(option)]
        /// direction
        _direction: String,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &["5", "--direction", "north"]).unwrap();
    assert_eq!(actual, &["program-name", "speed", "--direction"]);
}

#[test]
fn redact_arg_values_positional_optional_option() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(positional, arg_name = "speed")]
        /// speed of cmd
        _speed: u8,

        #[argy(option)]
        /// direction
        _direction: Option<String>,
    }

    let actual = Cmd::redact_arg_values(&["program-name"], &["5"]).unwrap();
    assert_eq!(actual, &["program-name", "speed"]);
}

#[test]
fn redact_arg_values_subcommand() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(positional, arg_name = "speed")]
        /// speed of cmd
        _speed: u8,

        #[argy(subcommand)]
        /// means of transportation
        _means: MeansSubcommand,
    }

    #[allow(dead_code)]
    #[derive(FromArgs, Debug)]
    /// Short description
    #[argy(subcommand)]
    enum MeansSubcommand {
        Walking(WalkingSubcommand),
        Biking(BikingSubcommand),
        Driving(DrivingSubcommand),
    }

    #[derive(FromArgs, Debug)]
    #[argy(subcommand, name = "walking")]
    /// Short description
    struct WalkingSubcommand {
        #[argy(option)]
        /// a song to listen to
        _music: String,
    }

    #[derive(FromArgs, Debug)]
    #[argy(subcommand, name = "biking")]
    /// Short description
    struct BikingSubcommand {}
    #[derive(FromArgs, Debug)]
    #[argy(subcommand, name = "driving")]
    /// short description
    struct DrivingSubcommand {}

    let actual =
        Cmd::redact_arg_values(&["program-name"], &["5", "walking", "--music", "Bach"]).unwrap();
    assert_eq!(actual, &["program-name", "speed", "walking", "--music"]);
}

#[test]
fn redact_arg_values_subcommand_with_space_in_name() {
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(positional, arg_name = "speed")]
        /// speed of cmd
        _speed: u8,

        #[argy(subcommand)]
        /// means of transportation
        _means: MeansSubcommand,
    }

    #[allow(dead_code)]
    #[derive(FromArgs, Debug)]
    /// Short description
    #[argy(subcommand)]
    enum MeansSubcommand {
        Walking(WalkingSubcommand),
        Biking(BikingSubcommand),
    }

    #[derive(FromArgs, Debug)]
    #[argy(subcommand, name = "has space")]
    /// Short description
    struct WalkingSubcommand {
        #[argy(option)]
        /// a song to listen to
        _music: String,
    }

    #[derive(FromArgs, Debug)]
    #[argy(subcommand, name = "biking")]
    /// Short description
    struct BikingSubcommand {}

    let actual =
        Cmd::redact_arg_values(&["program-name"], &["5", "has space", "--music", "Bach"]).unwrap();
    assert_eq!(actual, &["program-name", "speed", "has space", "--music"]);
}

#[test]
#[cfg(feature = "help")]
fn redact_arg_values_produces_help() {
    #[derive(argy::FromArgs, Debug, PartialEq)]
    /// Woot
    struct Repeating {
        #[argy(option, short = 'n')]
        /// fooey
        n: Vec<String>,
    }

    assert_eq!(
        Repeating::redact_arg_values(&["program-name"], &["--help"]),
        Err(argy::EarlyExit {
            output: r"Usage: program-name [-n <n...>]

Woot

Options:
  -n, --n       fooey
  --help, help  display usage information
"
            .to_owned(),
            status: Ok(()),
        }),
    );
}

#[test]
fn redact_arg_values_produces_errors_with_bad_arguments() {
    #[derive(argy::FromArgs, Debug, PartialEq)]
    /// Woot
    struct Cmd {
        #[argy(option, short = 'n')]
        /// fooey
        n: String,
    }

    assert_eq!(
        Cmd::redact_arg_values(&["program-name"], &["--n"]),
        Err(argy::EarlyExit {
            output: "No value provided for option '--n'.\n".to_owned(),
            status: Err(()),
        }),
    );
}

#[test]
fn redact_arg_values_does_not_warn_if_used() {
    #[forbid(unused)]
    #[derive(FromArgs, Debug)]
    /// Short description
    struct Cmd {
        #[argy(positional)]
        /// speed of cmd
        speed: u8,
    }

    let cmd = Cmd::from_args(&["program-name"], &["5"]).unwrap();
    assert_eq!(cmd.speed, 5);

    let actual = Cmd::redact_arg_values(&["program-name"], &["5"]).unwrap();
    assert_eq!(actual, &["program-name", "speed"]);
}

#[test]
fn subcommand_does_not_panic() {
    #[derive(FromArgs, PartialEq, Debug)]
    #[argy(subcommand)]
    enum SubCommandEnum {
        Cmd(SubCommand),
        CmdTwo(SubCommandTwo),
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// First subcommand.
    #[argy(subcommand, name = "one")]
    struct SubCommand {
        #[argy(positional)]
        /// how many x
        x: usize,
    }

    #[derive(FromArgs, PartialEq, Debug)]
    /// Second subcommand.
    #[argy(subcommand, name = "two")]
    struct SubCommandTwo {
        #[argy(switch)]
        /// whether to fooey
        fooey: bool,
    }

    // Passing no subcommand name to an emum
    assert_eq!(
        SubCommandEnum::from_args(&[], &["5"]).unwrap_err(),
        argy::EarlyExit { output: "no subcommand name".into(), status: Err(()) },
    );

    assert_eq!(
        SubCommandEnum::redact_arg_values(&[], &["5"]).unwrap_err(),
        argy::EarlyExit { output: "no subcommand name".into(), status: Err(()) },
    );

    // Passing unknown subcommand name to an emum
    assert_eq!(
        SubCommandEnum::from_args(&["fooey"], &["5"]).unwrap_err(),
        argy::EarlyExit { output: "no subcommand matched".into(), status: Err(()) },
    );

    assert_eq!(
        SubCommandEnum::redact_arg_values(&["fooey"], &["5"]).unwrap_err(),
        argy::EarlyExit { output: "no subcommand matched".into(), status: Err(()) },
    );

    // Passing unknown subcommand name to a struct
    assert_eq!(
        SubCommand::redact_arg_values(&[], &["5"]).unwrap_err(),
        argy::EarlyExit { output: "no subcommand name".into(), status: Err(()) },
    );
}

#[test]
fn long_alphanumeric() {
    #[derive(FromArgs)]
    /// Short description
    struct Cmd {
        #[argy(option, long = "ac97")]
        /// fooey
        ac97: String,
    }

    let cmd = Cmd::from_args(&["cmdname"], &["--ac97", "bar"]).unwrap();
    assert_eq!(cmd.ac97, "bar");
}

#[test]
#[cfg(feature = "help")]
fn override_usage() {
    /// Height options
    #[derive(FromArgs)]
    #[argy(help_triggers("-h", "--help", "help"))]
    #[argy(usage = "USAGE LINE")]
    struct Height {
        /// how high to go
        #[argy(option)]
        _height: usize,
    }

    assert_help_string::<Height>(
        r"Usage: test_arg_0 USAGE LINE

Height options

Options:
  --height          how high to go
  -h, --help, help  display usage information
",
    );
}

#[test]
#[cfg(feature = "help")]
fn customize_usage() {
    /// Height options
    #[derive(FromArgs)]
    #[argy(help_triggers("-h", "--help", "help"))]
    struct Height {
        /// how high to go
        #[argy(option)]
        #[argy(usage)]
        _height: usize,

        /// hidden from usage
        #[argy(option)]
        _hidden: usize,
    }

    assert_help_string::<Height>(
        r"Usage: test_arg_0 --height <height>

Height options

Options:
  --height          how high to go
  --hidden          hidden from usage
  -h, --help, help  display usage information
",
    );
}

#[test]
#[cfg(feature = "help")]
fn optional_value_switch_usage_marker() {
    #[derive(FromArgs)]
    /// Has an optional-value switch.
    struct Cmd {
        /// an optional boolean switch
        #[argy(switch)]
        _flag: Option<bool>,
    }

    assert_help_string::<Cmd>(
        r"Usage: test_arg_0 [--flag[=<bool>]]

Has an optional-value switch.

Options:
  --flag        an optional boolean switch
  --help, help  display usage information
",
    );
}

#[test]
fn conflicts_with_rejects_mutually_exclusive_options() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Query.
    struct Query {
        /// interactive mode
        #[argy(switch, short = 'i', conflicts_with = "list")]
        interactive: bool,
        /// list mode
        #[argy(switch, short = 'l')]
        list: bool,
        /// verbose mode
        #[argy(switch, short = 'v')]
        verbose: bool,
    }

    // A non-conflicting pair parses normally.
    let ok = Query::from_args(&["query"], &["--interactive", "--verbose"])
        .expect("non-conflicting pair should parse");
    assert_eq!(ok, Query { interactive: true, list: false, verbose: true });

    // Passing both options of a conflicting pair errors, naming both.
    let e = Query::from_args(&["query"], &["--interactive", "--list"])
        .expect_err("conflicting pair should error");
    assert!(e.status.is_err());
    assert!(e.output.contains("--interactive"), "unexpected: {:?}", e.output);
    assert!(e.output.contains("--list"), "unexpected: {:?}", e.output);

    // The order of the conflicting options does not matter.
    let e = Query::from_args(&["query"], &["-l", "-i"]).expect_err("conflicting pair should error");
    assert!(e.status.is_err());
    assert!(e.output.contains("--interactive"), "unexpected: {:?}", e.output);
    assert!(e.output.contains("--list"), "unexpected: {:?}", e.output);
}

#[test]
fn requires_enforces_single_form() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Connect.
    struct Connect {
        /// target host
        #[argy(option, requires = "user")]
        host: Option<String>,
        /// user name
        #[argy(option)]
        user: Option<String>,
    }

    // Providing the option without its required option is a usage error.
    let e = Connect::from_args(&["connect"], &["--host", "example.com"])
        .expect_err("host requires user");
    assert!(e.status.is_err());
    assert!(e.output.contains("Required options not provided:"), "unexpected: {:?}", e.output);
    assert!(e.output.contains("--user"), "unexpected: {:?}", e.output);

    // Providing both parses normally.
    let ok = Connect::from_args(&["connect"], &["--host", "example.com", "--user", "alice"])
        .expect("both options should parse");
    assert_eq!(ok, Connect { host: Some("example.com".into()), user: Some("alice".into()) });

    // The option with no `requires` may appear alone.
    let ok =
        Connect::from_args(&["connect"], &["--user", "alice"]).expect("user alone should parse");
    assert_eq!(ok, Connect { host: None, user: Some("alice".into()) });
}

#[test]
fn requires_enforces_list_form() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Deploy.
    struct Deploy {
        /// env name
        #[argy(option, requires = ["region", "token"])]
        env: Option<String>,
        /// region
        #[argy(option)]
        region: Option<String>,
        /// auth token
        #[argy(option)]
        token: Option<String>,
    }

    // Missing one of the required options reports that specific option.
    let e = Deploy::from_args(&["deploy"], &["--env", "prod", "--region", "us-east-1"])
        .expect_err("env requires token too");
    assert!(e.status.is_err());
    assert!(e.output.contains("--token"), "unexpected: {:?}", e.output);

    // Missing both required options reports both.
    let e = Deploy::from_args(&["deploy"], &["--env", "prod"])
        .expect_err("env requires region and token");
    assert!(e.status.is_err());
    assert!(e.output.contains("--region"), "unexpected: {:?}", e.output);
    assert!(e.output.contains("--token"), "unexpected: {:?}", e.output);

    // Providing all required options parses normally.
    let ok =
        Deploy::from_args(&["deploy"], &["--env", "prod", "--region", "us-east-1", "--token", "t"])
            .expect("all options should parse");
    assert_eq!(
        ok,
        Deploy {
            env: Some("prod".into()),
            region: Some("us-east-1".into()),
            token: Some("t".into()),
        }
    );
}

#[test]
fn requires_supports_mutual_requirements() {
    #[derive(FromArgs, PartialEq, Debug)]
    /// Pair.
    struct Pair {
        /// first
        #[argy(option, requires = "second")]
        first: Option<String>,
        /// second
        #[argy(option, requires = "first")]
        second: Option<String>,
    }

    // Providing only the first reports the second as missing.
    let e = Pair::from_args(&["pair"], &["--first", "a"]).expect_err("first requires second");
    assert!(e.status.is_err());
    assert!(e.output.contains("--second"), "unexpected: {:?}", e.output);

    // Providing only the second reports the first as missing.
    let e = Pair::from_args(&["pair"], &["--second", "b"]).expect_err("second requires first");
    assert!(e.status.is_err());
    assert!(e.output.contains("--first"), "unexpected: {:?}", e.output);

    // Providing both parses normally.
    let ok =
        Pair::from_args(&["pair"], &["--first", "a", "--second", "b"]).expect("both should parse");
    assert_eq!(ok, Pair { first: Some("a".into()), second: Some("b".into()) });
}
