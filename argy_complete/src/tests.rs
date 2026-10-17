// Copyright (c) 2026 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use crate::Generator;
use argy_shared::{CommandInfoWithArgs, FlagInfo, FlagInfoKind, Optionality, SubCommandInfo};

fn make_mock_command() -> CommandInfoWithArgs<'static> {
    let subcmd_test_list =
        CommandInfoWithArgs { name: "list", description: "a list command", ..Default::default() };
    let subcmd_test_run =
        CommandInfoWithArgs { name: "run", description: "a run command", ..Default::default() };
    let subcmd_test = CommandInfoWithArgs {
        name: "test",
        description: "a test command inside subcmd",
        commands: vec![
            SubCommandInfo { name: "list", command: subcmd_test_list },
            SubCommandInfo { name: "run", command: subcmd_test_run },
        ],
        ..Default::default()
    };
    let subcmd_help =
        CommandInfoWithArgs { name: "help", description: "a help command", ..Default::default() };

    let subcmd_info = CommandInfoWithArgs {
        name: "subcmd",
        description: "a sub command",
        commands: vec![
            SubCommandInfo { name: "help", command: subcmd_help },
            SubCommandInfo { name: "test", command: subcmd_test },
        ],
        ..Default::default()
    };

    let test_run_info =
        CommandInfoWithArgs { name: "run", description: "a run command", ..Default::default() };
    let test_build_info =
        CommandInfoWithArgs { name: "build", description: "a build command", ..Default::default() };
    let nested_subcmd_info = CommandInfoWithArgs {
        name: "test",
        description: "a test command",
        commands: vec![
            SubCommandInfo { name: "run", command: test_run_info },
            SubCommandInfo { name: "build", command: test_build_info },
        ],
        ..Default::default()
    };

    CommandInfoWithArgs {
        name: "mycmd",
        description: "A standard command",
        flags: &[FlagInfo {
            kind: FlagInfoKind::Switch,
            optionality: Optionality::Optional,
            long: "--verbose",
            short: Some('v'),
            description: "verbose output",
            hidden: false,
            aliases: &[],
        }],
        commands: vec![
            SubCommandInfo { name: "subcmd", command: subcmd_info },
            SubCommandInfo { name: "test", command: nested_subcmd_info },
        ],
        ..Default::default()
    }
}

#[test]
fn test_bash_generator() {
    let cmd = make_mock_command();
    let bash_out = crate::bash::Bash::generate("mycmd", &cmd);

    assert!(bash_out.contains("cmd=\"mycmd_subcmd\""));
    assert!(bash_out.contains("opts=\"--verbose -v\""));
    assert!(bash_out.contains("cmds=\"help test\""), "bash should have updated cmds for subcmd");
}

#[test]
fn test_zsh_generator() {
    let cmd = make_mock_command();
    let zsh_out = crate::zsh::Zsh::generate("mycmd", &cmd);

    assert!(zsh_out.contains("#compdef mycmd"));
    assert!(zsh_out.contains("'*::command:->subcmd'"));
    assert!(zsh_out.contains("(-v --verbose)")); // Simplified check
}

#[test]
fn test_fish_generator() {
    let cmd = make_mock_command();
    let fish_out = crate::fish::Fish::generate("mycmd", &cmd);

    assert!(fish_out.contains("function __fish_mycmd_using_command"));
    assert!(fish_out.contains(
        "complete -c mycmd -n '__fish_mycmd_using_command' -f -l verbose -s v -d 'verbose output'"
    ));
    assert!(fish_out.contains(
        "complete -c mycmd -n '__fish_mycmd_using_command' -f -a 'subcmd' -d 'a sub command'"
    ));
}

#[test]
fn test_nushell_generator() {
    let cmd = make_mock_command();
    let nushell_out = crate::nushell::Nushell::generate("mycmd", &cmd);

    assert!(nushell_out.contains("export extern \"mycmd\" ["));
    assert!(nushell_out.contains("--verbose(-v) # verbose output"));
    assert!(nushell_out.contains("export extern \"mycmd subcmd\" ["));
}

#[test]
fn test_hidden_subcommand_omitted_from_completions() {
    let visible_cmd = CommandInfoWithArgs {
        name: "visible",
        description: "a visible command",
        ..Default::default()
    };
    let hidden_cmd = CommandInfoWithArgs {
        name: "hidden",
        description: "an internal command",
        hidden: true,
        ..Default::default()
    };
    let cmd = CommandInfoWithArgs {
        name: "mycmd",
        description: "A command",
        commands: vec![
            SubCommandInfo { name: "visible", command: visible_cmd },
            SubCommandInfo { name: "hidden", command: hidden_cmd },
        ],
        ..Default::default()
    };

    let bash_out = crate::bash::Bash::generate("mycmd", &cmd);
    assert!(bash_out.contains("visible"), "bash should list visible subcommand");
    assert!(!bash_out.contains("hidden"), "bash should omit hidden subcommand");

    let zsh_out = crate::zsh::Zsh::generate("mycmd", &cmd);
    assert!(zsh_out.contains("visible"), "zsh should list visible subcommand");
    assert!(!zsh_out.contains("hidden"), "zsh should omit hidden subcommand");

    let fish_out = crate::fish::Fish::generate("mycmd", &cmd);
    assert!(fish_out.contains("'visible'"), "fish should list visible subcommand");
    assert!(!fish_out.contains("'hidden'"), "fish should omit hidden subcommand");

    let nushell_out = crate::nushell::Nushell::generate("mycmd", &cmd);
    assert!(nushell_out.contains("mycmd visible"), "nushell should list visible subcommand");
    assert!(!nushell_out.contains("mycmd hidden"), "nushell should omit hidden subcommand");
}

fn make_escaping_command() -> CommandInfoWithArgs<'static> {
    let sub = CommandInfoWithArgs {
        name: "sub",
        description: "line one\nit's got a quote",
        ..Default::default()
    };
    CommandInfoWithArgs {
        name: "mycmd",
        description: "A command",
        flags: &[FlagInfo {
            kind: FlagInfoKind::Switch,
            optionality: Optionality::Optional,
            long: "--flag",
            short: None,
            description: "flag one\nit's a flag",
            hidden: false,
            aliases: &[],
        }],
        commands: vec![SubCommandInfo { name: "sub", command: sub }],
        ..Default::default()
    }
}

#[test]
fn test_zsh_description_escaping() {
    let cmd = make_escaping_command();
    let zsh_out = crate::zsh::Zsh::generate("mycmd", &cmd);

    // Newline collapsed, single quote escaped zsh-style ('\''), inside [{desc}].
    assert!(
        zsh_out.contains("[flag one it'\\''s a flag]"),
        "zsh flag description should be escaped, got: {}",
        zsh_out
    );
    // Subcommand description inside 'name:desc'.
    assert!(
        zsh_out.contains("'sub:line one it'\\''s got a quote'"),
        "zsh subcommand description should be escaped, got: {}",
        zsh_out
    );
    // The embedded newlines must not survive into the emitted script.
    assert!(!zsh_out.contains("flag one\n"), "zsh flag newline breakout");
    assert!(!zsh_out.contains("line one\n"), "zsh subcommand newline breakout");
}

#[test]
fn test_fish_description_escaping() {
    let cmd = make_escaping_command();
    let fish_out = crate::fish::Fish::generate("mycmd", &cmd);

    // Newline collapsed, single quote escaped with backslash, inside -d '...'.
    assert!(
        fish_out.contains("-l flag -d 'flag one it\\'s a flag'"),
        "fish flag description should be escaped, got: {}",
        fish_out
    );
    assert!(
        fish_out.contains("-a 'sub' -d 'line one it\\'s got a quote'"),
        "fish subcommand description should be escaped, got: {}",
        fish_out
    );
    assert!(!fish_out.contains("flag one\n"), "fish flag newline breakout");
    assert!(!fish_out.contains("line one\n"), "fish subcommand newline breakout");
}

#[test]
fn test_nushell_description_escaping() {
    let cmd = make_escaping_command();
    let nushell_out = crate::nushell::Nushell::generate("mycmd", &cmd);

    // Newline collapsed, single quote escaped, inside a '# ' comment.
    assert!(
        nushell_out.contains("--flag # flag one it\\'s a flag"),
        "nushell flag description should be escaped, got: {}",
        nushell_out
    );
    assert!(!nushell_out.contains("flag one\n"), "nushell flag newline breakout");
}
