#![cfg(test)]
// Copyright (c) 2023 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use argy::{
    ArgsInfo, CommandInfoWithArgs, ErrorCodeInfo, FlagInfo, FlagInfoKind, FromArgs, Optionality,
    PositionalInfo, SubCommandInfo,
};

fn assert_args_info<T: ArgsInfo>(expected: &CommandInfoWithArgs) {
    let actual_value = T::get_args_info();
    assert_eq!(expected, &actual_value);
}

const HELP_FLAG: FlagInfo<'_> = FlagInfo {
    kind: FlagInfoKind::Switch,
    optionality: Optionality::Optional,
    long: "--help",
    short: None,
    description: "display usage information",
    hidden: false,
    aliases: &[],
};

/// Tests that exercise the JSON output for help text.
#[test]
fn args_info_test_subcommand() {
    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        #[argy(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    #[argy(subcommand)]
    enum MySubCommandEnum {
        One(SubCommandOne),
        Two(SubCommandTwo),
    }

    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    /// First subcommand.
    #[argy(subcommand, name = "one")]
    struct SubCommandOne {
        #[argy(option)]
        /// how many x
        x: usize,
    }

    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    /// Second subcommand.
    #[argy(subcommand, name = "two")]
    struct SubCommandTwo {
        #[argy(switch)]
        /// whether to fooey
        fooey: bool,
    }

    let command_one = CommandInfoWithArgs {
        name: "one",
        short: &'\0',
        description: "First subcommand.",
        flags: &[
            HELP_FLAG,
            FlagInfo {
                kind: FlagInfoKind::Option { arg_name: "x" },
                optionality: Optionality::Required,
                long: "--x",
                short: None,
                description: "how many x",
                hidden: false,
                aliases: &[],
            },
        ],
        ..Default::default()
    };

    assert_args_info::<TopLevel>(&CommandInfoWithArgs {
        name: "TopLevel",
        short: &'\0',
        description: "Top-level command.",
        examples: &[],
        flags: &[HELP_FLAG],
        notes: &[],
        positionals: &[],
        error_codes: &[],
        hidden: false,
        aliases: &[],
        commands: vec![
            SubCommandInfo { name: "one", command: command_one.clone() },
            SubCommandInfo {
                name: "two",
                command: CommandInfoWithArgs {
                    name: "two",
                    short: &'\0',
                    description: "Second subcommand.",
                    flags: &[
                        HELP_FLAG,
                        FlagInfo {
                            kind: FlagInfoKind::Switch,
                            optionality: Optionality::Optional,
                            long: "--fooey",
                            short: None,
                            description: "whether to fooey",
                            hidden: false,
                            aliases: &[],
                        },
                    ],
                    ..Default::default()
                },
            },
        ],
    });

    assert_args_info::<SubCommandOne>(&command_one);
}

#[test]
fn args_info_test_multiline_doc_comment() {
    #[derive(FromArgs, ArgsInfo)]
    /// Short description
    struct Cmd {
        #[argy(switch)]
        /// a switch with a description
        /// that is spread across
        /// a number of
        /// lines of comments.
        _s: bool,
    }
    assert_args_info::<Cmd>(
            &CommandInfoWithArgs {
                name: "Cmd",
                short: &'\0',
                description: "Short description",
                flags: &[HELP_FLAG,
                FlagInfo {
                    kind: FlagInfoKind::Switch,
                    optionality: Optionality::Optional,
                    long: "--s",
                    short: None,
                    description: "a switch with a description that is spread across a number of lines of comments.",
                    hidden:false,
                                aliases: &[],
}
                ],
           ..Default::default()
            });
}

#[test]
fn args_info_test_basic_args() {
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    /// Basic command args demonstrating multiple types and cardinality. "With quotes"
    struct Basic {
        /// should the power be on. "Quoted value" should work too.
        #[argy(switch)]
        power: bool,

        /// option that is required because of no default and not Option<>.
        #[argy(option, long = "required")]
        required_flag: String,

        /// optional speed if not specified it is None.
        #[argy(option, short = 's')]
        speed: Option<u8>,

        /// repeatable option.
        #[argy(option, arg_name = "url")]
        link: Vec<String>,
    }
    assert_args_info::<Basic>(&CommandInfoWithArgs {
        name: "Basic",
        short: &'\0',
        description:
            "Basic command args demonstrating multiple types and cardinality. \"With quotes\"",
        flags: &[
            FlagInfo {
                kind: FlagInfoKind::Switch,
                optionality: Optionality::Optional,
                long: "--help",
                short: None,
                description: "display usage information",
                hidden: false,
                aliases: &[],
            },
            FlagInfo {
                kind: FlagInfoKind::Switch,
                optionality: Optionality::Optional,
                long: "--power",
                short: None,
                description: "should the power be on. \"Quoted value\" should work too.",
                hidden: false,
                aliases: &[],
            },
            FlagInfo {
                kind: FlagInfoKind::Option { arg_name: "required" },
                optionality: Optionality::Required,
                long: "--required",
                short: None,
                description: "option that is required because of no default and not Option<>.",
                hidden: false,
                aliases: &[],
            },
            FlagInfo {
                kind: FlagInfoKind::Option { arg_name: "speed" },
                optionality: Optionality::Optional,
                long: "--speed",
                short: Some('s'),
                description: "optional speed if not specified it is None.",
                hidden: false,
                aliases: &[],
            },
            FlagInfo {
                kind: FlagInfoKind::Option { arg_name: "url" },
                optionality: Optionality::Repeating,
                long: "--link",
                short: None,
                description: "repeatable option.",
                hidden: false,
                aliases: &[],
            },
        ],
        ..Default::default()
    });
}

#[test]
fn args_info_test_positional_args() {
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    /// Command with positional args demonstrating. "With quotes"
    struct Positional {
        /// the "root" position.
        #[argy(positional, arg_name = "root")]
        root_value: String,

        /// trunk value
        #[argy(positional)]
        trunk: String,

        /// leaves. There can be many leaves.
        #[argy(positional)]
        leaves: Vec<String>,
    }
    assert_args_info::<Positional>(&CommandInfoWithArgs {
        name: "Positional",
        short: &'\0',
        description: "Command with positional args demonstrating. \"With quotes\"",
        flags: &[HELP_FLAG],
        positionals: &[
            PositionalInfo {
                name: "root",
                description: "the \"root\" position.",
                optionality: Optionality::Required,
                hidden: false,
            },
            PositionalInfo {
                name: "trunk",
                description: "trunk value",
                optionality: Optionality::Required,
                hidden: false,
            },
            PositionalInfo {
                name: "leaves",
                description: "leaves. There can be many leaves.",
                optionality: Optionality::Repeating,
                hidden: false,
            },
        ],

        ..Default::default()
    });
}

#[test]
fn args_info_test_optional_positional_args() {
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    /// Command with positional args demonstrating last value is optional
    struct Positional {
        /// the "root" position.
        #[argy(positional, arg_name = "root")]
        root_value: String,

        /// trunk value
        #[argy(positional)]
        trunk: String,

        /// leaves. There can be an optional leaves.
        #[argy(positional)]
        leaves: Option<String>,
    }
    assert_args_info::<Positional>(&CommandInfoWithArgs {
        name: "Positional",
        short: &'\0',
        description: "Command with positional args demonstrating last value is optional",
        flags: &[FlagInfo {
            kind: FlagInfoKind::Switch,
            optionality: Optionality::Optional,
            long: "--help",
            short: None,
            description: "display usage information",
            hidden: false,
            aliases: &[],
        }],
        positionals: &[
            PositionalInfo {
                name: "root",
                description: "the \"root\" position.",
                optionality: Optionality::Required,
                hidden: false,
            },
            PositionalInfo {
                name: "trunk",
                description: "trunk value",
                optionality: Optionality::Required,
                hidden: false,
            },
            PositionalInfo {
                name: "leaves",
                description: "leaves. There can be an optional leaves.",
                optionality: Optionality::Optional,
                hidden: false,
            },
        ],

        ..Default::default()
    });
}

#[test]
fn args_info_test_default_positional_args() {
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    /// Command with positional args demonstrating last value is defaulted.
    struct Positional {
        /// the "root" position.
        #[argy(positional, arg_name = "root")]
        root_value: String,

        /// trunk value
        #[argy(positional)]
        trunk: String,

        /// leaves. There can be one leaf, defaults to hello.
        #[argy(positional, default = "String::from(\"hello\")")]
        leaves: String,
    }
    assert_args_info::<Positional>(&CommandInfoWithArgs {
        name: "Positional",
        short: &'\0',
        description: "Command with positional args demonstrating last value is defaulted.",
        flags: &[HELP_FLAG],
        positionals: &[
            PositionalInfo {
                name: "root",
                description: "the \"root\" position.",
                optionality: Optionality::Required,
                hidden: false,
            },
            PositionalInfo {
                name: "trunk",
                description: "trunk value",
                optionality: Optionality::Required,
                hidden: false,
            },
            PositionalInfo {
                name: "leaves",
                description: "leaves. There can be one leaf, defaults to hello.",
                optionality: Optionality::Optional,
                hidden: false,
            },
        ],

        ..Default::default()
    });
}

#[test]
fn args_info_test_notes_examples_errors() {
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    /// Command with Examples and usage Notes, including error codes.
    #[argy(
        note = r#"
    These usage notes appear for {command_name} and how to best use it.
    The formatting should be preserved.
    one
    two
    three then a blank
    
    and one last line with "quoted text"."#,
        example = r#"
    Use the command with 1 file:
    `{command_name} /path/to/file`
    Use it with a "wildcard":
    `{command_name} /path/to/*`
     a blank line
    
    and one last line with "quoted text"."#,
        error_code(0, "Success"),
        error_code(1, "General Error"),
        error_code(2, "Some error with \"quotes\"")
    )]
    struct NotesExamplesErrors {
        /// the "root" position.
        #[argy(positional, arg_name = "files")]
        fields: Vec<std::path::PathBuf>,
    }
    assert_args_info::<NotesExamplesErrors>(
            &CommandInfoWithArgs {
                name: "NotesExamplesErrors",
                short: &'\0',
                description: "Command with Examples and usage Notes, including error codes.",
                examples: &["\n    Use the command with 1 file:\n    `{command_name} /path/to/file`\n    Use it with a \"wildcard\":\n    `{command_name} /path/to/*`\n     a blank line\n    \n    and one last line with \"quoted text\"."],
                flags: &[HELP_FLAG
                ],
                positionals: &[
                    PositionalInfo{
                        name: "files",
                        description: "the \"root\" position.",
                        optionality: Optionality::Repeating,
                        hidden:false
                    }
                ],
                notes: &["\n    These usage notes appear for {command_name} and how to best use it.\n    The formatting should be preserved.\n    one\n    two\n    three then a blank\n    \n    and one last line with \"quoted text\"."],
                error_codes: & [ErrorCodeInfo { code: 0, description: "Success" }, ErrorCodeInfo { code: 1, description: "General Error" }, ErrorCodeInfo { code: 2, description: "Some error with \"quotes\"" }],
                ..Default::default()
            });
}

#[test]
#[allow(clippy::too_many_lines)] // long test exercising many subcommand args_info outputs
fn args_info_test_subcommands() {
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    ///Top level command with "subcommands".
    struct TopLevel {
        /// show verbose output
        #[argy(switch)]
        verbose: bool,

        /// this doc comment does not appear anywhere.
        #[argy(subcommand)]
        cmd: SubcommandEnum,
    }

    #[derive(FromArgs, ArgsInfo)]
    #[allow(dead_code)]
    #[argy(subcommand)]
    /// Doc comments for subcommand enums does not appear in the help text.
    enum SubcommandEnum {
        Command1(Command1Args),
        Command2(Command2Args),
        Command3(Command3Args),
    }

    /// Command1 args are used for Command1.
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    #[argy(subcommand, name = "one")]
    struct Command1Args {
        /// the "root" position.
        #[argy(positional, arg_name = "root")]
        root_value: String,

        /// trunk value
        #[argy(positional)]
        trunk: String,

        /// leaves. There can be zero leaves, defaults to hello.
        #[argy(positional, default = "String::from(\"hello\")")]
        leaves: String,
    }
    /// Command2 args are used for Command2.
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    #[argy(subcommand, name = "two")]
    struct Command2Args {
        /// should the power be on. "Quoted value" should work too.
        #[argy(switch)]
        power: bool,

        /// option that is required because of no default and not Option<>.
        #[argy(option, long = "required")]
        required_flag: String,

        /// optional speed if not specified it is None.
        #[argy(option, short = 's')]
        speed: Option<u8>,

        /// repeatable option.
        #[argy(option, arg_name = "url")]
        link: Vec<String>,
    }
    /// Command3 args are used for Command3 which has no options or arguments.
    #[derive(FromArgs, ArgsInfo)]
    #[argy(subcommand, name = "three")]
    struct Command3Args {}

    assert_args_info::<TopLevel>(&CommandInfoWithArgs {
        name: "TopLevel",
        short: &'\0',
        description: "Top level command with \"subcommands\".",
        flags: &[
            HELP_FLAG,
            FlagInfo {
                kind: FlagInfoKind::Switch,
                optionality: Optionality::Optional,
                long: "--verbose",
                short: None,
                description: "show verbose output",
                hidden: false,
                aliases: &[],
            },
        ],
        positionals: &[],
        commands: vec![
            SubCommandInfo {
                name: "one",
                command: CommandInfoWithArgs {
                    name: "one",
                    short: &'\0',
                    description: "Command1 args are used for Command1.",
                    flags: &[HELP_FLAG],
                    positionals: &[
                        PositionalInfo {
                            name: "root",
                            description: "the \"root\" position.",
                            optionality: Optionality::Required,
                            hidden: false,
                        },
                        PositionalInfo {
                            name: "trunk",
                            description: "trunk value",
                            optionality: Optionality::Required,
                            hidden: false,
                        },
                        PositionalInfo {
                            name: "leaves",
                            description: "leaves. There can be zero leaves, defaults to hello.",
                            optionality: Optionality::Optional,
                            hidden: false,
                        },
                    ],
                    ..Default::default()
                },
            },
            SubCommandInfo {
                name: "two",
                command: CommandInfoWithArgs {
                    name: "two",
                    short: &'\0',
                    description: "Command2 args are used for Command2.",
                    flags: &[
                        HELP_FLAG,
                        FlagInfo {
                            kind: FlagInfoKind::Switch,
                            optionality: Optionality::Optional,
                            long: "--power",
                            short: None,
                            description:
                                "should the power be on. \"Quoted value\" should work too.",
                            hidden: false,
                            aliases: &[],
                        },
                        FlagInfo {
                            kind: FlagInfoKind::Option { arg_name: "required" },
                            optionality: Optionality::Required,
                            long: "--required",
                            short: None,
                            description:
                                "option that is required because of no default and not Option<>.",
                            hidden: false,
                            aliases: &[],
                        },
                        FlagInfo {
                            kind: FlagInfoKind::Option { arg_name: "speed" },
                            optionality: Optionality::Optional,
                            long: "--speed",
                            short: Some('s'),
                            description: "optional speed if not specified it is None.",
                            hidden: false,
                            aliases: &[],
                        },
                        FlagInfo {
                            kind: FlagInfoKind::Option { arg_name: "url" },
                            optionality: Optionality::Repeating,
                            long: "--link",
                            short: None,
                            description: "repeatable option.",
                            hidden: false,
                            aliases: &[],
                        },
                    ],
                    ..Default::default()
                },
            },
            SubCommandInfo {
                name: "three",
                command: CommandInfoWithArgs {
                    name: "three",
                    short: &'\0',
                    description:
                        "Command3 args are used for Command3 which has no options or arguments.",
                    flags: &[HELP_FLAG],
                    positionals: &[],
                    ..Default::default()
                },
            },
        ],
        ..Default::default()
    });
}

#[test]
fn args_info_test_subcommand_notes_examples() {
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    ///Top level command with "subcommands".
    #[argy(
        note = "Top level note",
        example = "Top level example",
        error_code(0, "Top level success")
    )]
    struct TopLevel {
        /// this doc comment does not appear anywhere.
        #[argy(subcommand)]
        cmd: SubcommandEnum,
    }

    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    #[argy(subcommand)]
    /// Doc comments for subcommand enums does not appear in the help text.
    enum SubcommandEnum {
        Command1(Command1Args),
    }

    /// Command1 args are used for subcommand one.
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    #[argy(
        subcommand,
        name = "one",
        note = "{command_name} is used as a subcommand of \"Top level\"",
        example = "\"Typical\" usage is `{command_name}`.",
        error_code(0, "one level success")
    )]
    struct Command1Args {
        /// the "root" position.
        #[argy(positional, arg_name = "root")]
        root_value: String,

        /// trunk value
        #[argy(positional)]
        trunk: String,

        /// leaves. There can be many leaves.
        #[argy(positional)]
        leaves: Vec<String>,
    }

    let command_one = CommandInfoWithArgs {
        name: "one",
        short: &'\0',
        description: "Command1 args are used for subcommand one.",
        error_codes: &[ErrorCodeInfo { code: 0, description: "one level success" }],
        examples: &["\"Typical\" usage is `{command_name}`."],
        flags: &[HELP_FLAG],
        notes: &["{command_name} is used as a subcommand of \"Top level\""],
        positionals: &[
            PositionalInfo {
                name: "root",
                description: "the \"root\" position.",
                optionality: Optionality::Required,
                hidden: false,
            },
            PositionalInfo {
                name: "trunk",
                description: "trunk value",
                optionality: Optionality::Required,
                hidden: false,
            },
            PositionalInfo {
                name: "leaves",
                description: "leaves. There can be many leaves.",
                optionality: Optionality::Repeating,
                hidden: false,
            },
        ],
        ..Default::default()
    };

    assert_args_info::<TopLevel>(&CommandInfoWithArgs {
        name: "TopLevel",
        short: &'\0',
        description: "Top level command with \"subcommands\".",
        error_codes: &[ErrorCodeInfo { code: 0, description: "Top level success" }],
        examples: &["Top level example"],
        flags: &[HELP_FLAG],
        notes: &["Top level note"],
        commands: vec![SubCommandInfo { name: "one", command: command_one.clone() }],
        ..Default::default()
    });

    assert_args_info::<Command1Args>(&command_one);
}

#[test]
fn args_info_test_example() {
    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    #[argy(
        description = "Destroy the contents of <file> with a specific \"method of destruction\".",
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

        #[allow(clippy::doc_markdown)] // doc text is rendered verbatim in args_info output
        /// say more. Defaults to $BLAST_VERBOSE.
        #[argy(switch, short = 'v')]
        verbose: bool,

        #[argy(subcommand)]
        command: HelpExampleSubCommands,
    }

    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    #[argy(subcommand)]
    enum HelpExampleSubCommands {
        BlowUp(BlowUp),
        Grind(GrindCommand),
    }

    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    #[argy(subcommand, name = "blow-up")]
    /// explosively separate
    struct BlowUp {
        /// blow up bombs safely
        #[argy(switch)]
        safely: bool,
    }

    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    #[argy(subcommand, name = "grind", description = "make smaller by many small cuts")]
    struct GrindCommand {
        /// wear a visor while grinding
        #[argy(switch)]
        safely: bool,
    }

    assert_args_info::<HelpExample>(
            &CommandInfoWithArgs {
                name: "HelpExample",
                short: &'\0',
                description: "Destroy the contents of <file> with a specific \"method of destruction\".",
                examples: &["Scribble 'abc' and then run |grind|.\n$ {command_name} -s 'abc' grind old.txt taxes.cp"],
                flags: &[HELP_FLAG,
                FlagInfo { kind: FlagInfoKind::Switch, optionality: Optionality::Optional, long: "--force", short: Some('f'), description: "force, ignore minor errors. This description is so long that it wraps to the next line.",
                hidden:false , aliases: &[]},
                FlagInfo { kind: FlagInfoKind::Switch, optionality: Optionality::Optional, long: "--really-really-really-long-name-for-pat", short: None, description: "documentation",
                hidden:false , aliases: &[]},
                FlagInfo { kind: FlagInfoKind::Option { arg_name: "scribble"},
                 optionality: Optionality::Required, long: "--scribble", short: Some('s'), description: "write <scribble> repeatedly",
                 hidden:false , aliases: &[]},
                  FlagInfo { kind: FlagInfoKind::Switch, optionality: Optionality::Optional, long: "--verbose", short: Some('v'), description: "say more. Defaults to $BLAST_VERBOSE.",
                  hidden:false , aliases: &[]}
                ],
                notes: &["Use `{command_name} help <command>` for details on [<args>] for a subcommand."],
                commands: vec![
                    SubCommandInfo { name: "blow-up",
                 command: CommandInfoWithArgs { name: "blow-up",
                  short: &'\0',
                  description: "explosively separate", 
                  flags:& [HELP_FLAG,
                   FlagInfo { kind: FlagInfoKind::Switch, optionality: Optionality::Optional, long: "--safely", short: None, description: "blow up bombs safely",
                   hidden:false , aliases: &[]}
                   ],
                ..Default::default()
             } },
              SubCommandInfo {
                 name: "grind",
                 command: CommandInfoWithArgs {
                     name: "grind",
                     description: "make smaller by many small cuts",
                     flags: &[HELP_FLAG,
                      FlagInfo { kind: FlagInfoKind::Switch, optionality: Optionality::Optional, long: "--safely", short: None, description: "wear a visor while grinding" ,hidden:false, aliases: &[]}],
                      ..Default::default()
                     }
                }],
                error_codes: &[ErrorCodeInfo { code: 2, description: "The blade is too dull." }, ErrorCodeInfo { code: 3, description: "Out of fuel." }],
                ..Default::default()
            }
            );
}

#[test]
fn positional_greedy() {
    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    /// Woot
    struct LastRepeatingGreedy {
        #[argy(positional)]
        /// fooey
        pub a: u32,
        #[argy(switch)]
        /// woo
        pub b: bool,
        #[argy(option)]
        /// stuff
        pub c: Option<String>,
        #[argy(positional, greedy)]
        /// fooey
        pub d: Vec<String>,
    }
    assert_args_info::<LastRepeatingGreedy>(&CommandInfoWithArgs {
        name: "LastRepeatingGreedy",
        short: &'\0',
        description: "Woot",
        flags: &[
            HELP_FLAG,
            FlagInfo {
                kind: FlagInfoKind::Switch,
                optionality: Optionality::Optional,
                long: "--b",
                short: None,
                description: "woo",
                hidden: false,
                aliases: &[],
            },
            FlagInfo {
                kind: FlagInfoKind::Option { arg_name: "c" },
                optionality: Optionality::Optional,
                long: "--c",
                short: None,
                description: "stuff",
                hidden: false,
                aliases: &[],
            },
        ],
        positionals: &[
            PositionalInfo {
                name: "a",
                description: "fooey",
                optionality: Optionality::Required,
                hidden: false,
            },
            PositionalInfo {
                name: "d",
                description: "fooey",
                optionality: Optionality::Greedy,
                hidden: false,
            },
        ],
        ..Default::default()
    });
}

#[test]
fn hidden_help_attribute() {
    #[derive(FromArgs, ArgsInfo)]
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

    assert_args_info::<Cmd>(&CommandInfoWithArgs {
        name: "Cmd",
        description: "Short description",
        flags: &[
            HELP_FLAG,
            FlagInfo {
                kind: FlagInfoKind::Option { arg_name: "three" },
                optionality: Optionality::Required,
                long: "--three",
                short: None,
                description: "this one should be hidden",
                hidden: true,
                aliases: &[],
            },
        ],
        positionals: &[
            PositionalInfo {
                name: "one",
                description: "this one should be hidden",
                optionality: Optionality::Required,
                hidden: true,
            },
            PositionalInfo {
                name: "two",
                description: "this one is real",
                optionality: Optionality::Required,
                hidden: false,
            },
        ],
        ..Default::default()
    });
}

#[test]
#[allow(clippy::too_many_lines)] // long test exercising dynamic subcommand args_info outputs
fn test_dynamic_subcommand() {
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

    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    /// Top-level command.
    struct TopLevel {
        #[argy(subcommand)]
        nested: MySubCommandEnum,
    }

    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    #[argy(subcommand)]
    enum MySubCommandEnum {
        One(SubCommandOne),
        Two(SubCommandTwo),
        #[argy(dynamic)]
        ThreeFourFive(DynamicSubCommandImpl),
    }

    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    /// First subcommand.
    #[argy(subcommand, name = "one")]
    struct SubCommandOne {
        #[argy(option)]
        /// how many x
        x: usize,
    }

    #[derive(FromArgs, ArgsInfo, PartialEq, Debug)]
    /// Second subcommand.
    #[argy(subcommand, name = "two")]
    struct SubCommandTwo {
        #[argy(switch)]
        /// whether to fooey
        fooey: bool,
    }

    assert_args_info::<TopLevel>(&CommandInfoWithArgs {
        name: "TopLevel",
        short: &'\0',
        description: "Top-level command.",
        flags: &[HELP_FLAG],
        commands: vec![
            SubCommandInfo {
                name: "one",
                command: CommandInfoWithArgs {
                    name: "one",
                    short: &'\0',
                    description: "First subcommand.",
                    flags: &[
                        HELP_FLAG,
                        FlagInfo {
                            kind: FlagInfoKind::Option { arg_name: "x" },
                            optionality: Optionality::Required,
                            long: "--x",
                            short: None,
                            description: "how many x",
                            hidden: false,
                            aliases: &[],
                        },
                    ],
                    ..Default::default()
                },
            },
            SubCommandInfo {
                name: "two",
                command: CommandInfoWithArgs {
                    name: "two",
                    short: &'\0',
                    description: "Second subcommand.",
                    flags: &[
                        HELP_FLAG,
                        FlagInfo {
                            kind: FlagInfoKind::Switch,
                            optionality: Optionality::Optional,
                            long: "--fooey",
                            short: None,
                            description: "whether to fooey",
                            hidden: false,
                            aliases: &[],
                        },
                    ],
                    ..Default::default()
                },
            },
            SubCommandInfo {
                name: "three",
                command: CommandInfoWithArgs {
                    name: "three",
                    short: &'\0',
                    description: "Third command",
                    ..Default::default()
                },
            },
            SubCommandInfo {
                name: "four",
                command: CommandInfoWithArgs {
                    name: "four",
                    short: &'\0',
                    description: "Fourth command",
                    ..Default::default()
                },
            },
            SubCommandInfo {
                name: "five",
                command: CommandInfoWithArgs {
                    name: "five",
                    short: &'\0',
                    description: "Fifth command",
                    ..Default::default()
                },
            },
        ],
        ..Default::default()
    });
}

#[test]
fn ok_hygiene() {
    #![allow(unused)]
    enum Foo {
        Ok,
        Err,
    }
    use Foo::*;

    #[derive(FromArgs)]
    /// A simple CLI tool
    struct Args {
        /// the input file
        #[argy(option)]
        input: String,
    }
}
/// Renders a command tree (like a `reference` subcommand) from the public
/// introspection API, exercising nested subcommands, options, switches,
/// positionals, and aliases.
#[test]
fn args_info_render_nested_reference_tree() {
    use std::fmt::Write;

    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    /// Reference a repository and its remotes.
    struct Repo {
        /// remote url to clone from
        #[argy(option)]
        url: String,
        /// recurse into submodules
        #[argy(switch, alias = "recursive")]
        recurse: bool,
        /// the command to run
        #[argy(subcommand)]
        cmd: RepoCmd,
    }

    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    #[argy(subcommand)]
    enum RepoCmd {
        Remote(RemoteCmd),
    }

    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    /// Manage git remotes.
    #[argy(subcommand, name = "remote", alias = "r")]
    struct RemoteCmd {
        /// remote name
        #[argy(positional)]
        name: String,
        /// remote subcommand
        #[argy(subcommand)]
        action: RemoteAction,
    }

    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    #[argy(subcommand)]
    enum RemoteAction {
        Add(AddCmd),
    }

    #[allow(dead_code)]
    #[derive(FromArgs, ArgsInfo)]
    /// Add a remote.
    #[argy(subcommand, name = "add", alias = "a")]
    struct AddCmd {
        /// fetch after add
        #[argy(switch, alias = "get")]
        fetch: bool,
    }

    fn render(out: &mut String, cmd: &CommandInfoWithArgs, depth: usize) {
        let indent = "  ".repeat(depth);
        writeln!(out, "{indent}## {}", cmd.name).unwrap();
        if !cmd.description.is_empty() {
            writeln!(out, "{indent}About: {}", cmd.description).unwrap();
        }
        if !cmd.aliases.is_empty() {
            writeln!(out, "{indent}Aliases: {}", cmd.aliases.join(", ")).unwrap();
        }
        for flag in cmd.flags {
            write!(out, "{indent}Flag {}", flag.long).unwrap();
            if let Some(short) = flag.short {
                write!(out, " (-{short})").unwrap();
            }
            if !flag.aliases.is_empty() {
                write!(out, " [aliases: {}]", flag.aliases.join(", ")).unwrap();
            }
            writeln!(out).unwrap();
        }
        for pos in cmd.positionals {
            writeln!(out, "{indent}Positional {}", pos.name).unwrap();
        }
        for sub in &cmd.commands {
            render(out, &sub.command, depth + 1);
        }
    }

    let mut rendered = String::new();
    render(&mut rendered, &Repo::get_args_info(), 0);

    // The top-level command: name, about, option, and a switch with an alias.
    assert!(rendered.contains("## Repo\nAbout: Reference a repository and its remotes."));
    assert!(rendered.contains("Flag --url"));
    assert!(rendered.contains("Flag --recurse [aliases: --recursive]"));
    // Nested subcommand with a command alias and a positional.
    assert!(rendered.contains("  ## remote\n  About: Manage git remotes.\n  Aliases: r"));
    assert!(rendered.contains("  Positional name"));
    // Deeper nesting: alias on a sub-subcommand and on a switch.
    assert!(rendered.contains("    ## add\n    About: Add a remote.\n    Aliases: a"));
    assert!(rendered.contains("Flag --fetch [aliases: --get]"));
}
