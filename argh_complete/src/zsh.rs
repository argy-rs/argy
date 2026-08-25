// Copyright (c) 2026 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Generation of completions for Zsh.

use crate::Generator;
use argh_shared::{CommandInfoWithArgs, FlagInfoKind};
use std::fmt::Write;

/// A generator for Zsh shell completions.
pub struct Zsh;

impl Generator for Zsh {
    fn generate(cmd_name: &str, cmd: &CommandInfoWithArgs<'_>) -> String {
        let mut out = String::new();

        writeln!(&mut out, "#compdef {cmd_name}").unwrap();
        writeln!(&mut out).unwrap();
        writeln!(&mut out, "_{cmd_name}() {{").unwrap();
        writeln!(&mut out, "    local context state state_descr line").unwrap();
        writeln!(&mut out, "    typeset -A opt_args").unwrap();
        writeln!(&mut out).unwrap();

        generate_zsh_args(&mut out, cmd_name, cmd, 1);

        writeln!(&mut out, "}}").unwrap();
        writeln!(&mut out).unwrap();

        // Generate functions for subcommands
        for subcmd in cmd.commands.iter().filter(|s| !s.command.hidden) {
            generate_zsh_subcmd(&mut out, cmd_name, &subcmd.command);
        }

        writeln!(&mut out, "if [[ $funcstack[1] == _{cmd_name} ]]; then").unwrap();
        writeln!(&mut out, "    _{cmd_name} \"$@\"").unwrap();
        writeln!(&mut out, "else").unwrap();
        writeln!(&mut out, "    compdef _{cmd_name} {cmd_name}").unwrap();
        writeln!(&mut out, "fi").unwrap();

        out
    }
}

fn generate_zsh_subcmd(out: &mut String, prefix: &str, cmd: &CommandInfoWithArgs<'_>) {
    let full_name = format!("{prefix}_{cmd_name}", cmd_name = cmd.name);
    writeln!(out, "_{full_name}() {{").unwrap();
    writeln!(out, "    local context state state_descr line").unwrap();
    writeln!(out, "    typeset -A opt_args").unwrap();
    writeln!(out).unwrap();

    generate_zsh_args(out, &full_name, cmd, 1);

    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    for subcmd in cmd.commands.iter().filter(|s| !s.command.hidden) {
        generate_zsh_subcmd(out, &full_name, &subcmd.command);
    }
}

fn generate_zsh_args(out: &mut String, prefix: &str, cmd: &CommandInfoWithArgs<'_>, indent: usize) {
    let ind = "    ".repeat(indent);
    writeln!(out, "{ind}_arguments -s -S \\").unwrap();

    for flag in cmd.flags {
        let mut def = String::new();
        let desc = flag
            .description
            .replace('[', "\\[")
            .replace(']', "\\]")
            .replace('\'', "'\\''")
            .replace(':', "\\:");

        let has_short = flag.short.is_some();
        let has_long = !flag.long.is_empty();

        if has_short && has_long {
            let short = format!("-{}", flag.short.unwrap());
            let _ = write!(def, "'({short} {long})'{{{short},{long}}}'[{desc}]'", long = flag.long);
        } else if has_long {
            let _ = write!(def, "'{long}[{desc}]'", long = flag.long);
        } else if has_short {
            let short = format!("-{}", flag.short.unwrap());
            let _ = write!(def, "'{short}[{desc}]'");
        }

        if let FlagInfoKind::Option { .. } = flag.kind {
            def.push_str("': :'"); // generic argument
        }

        writeln!(out, "{ind}    {def} \\").unwrap();
    }

    if cmd.commands.is_empty() {
        // Just cap it off if no subcommands
        writeln!(out, "{ind}    && return 0").unwrap();
    } else {
        writeln!(out, "{ind}    '*::command:->subcmd' && return 0").unwrap();
        writeln!(out).unwrap();
        writeln!(out, "{ind}case $state in").unwrap();
        writeln!(out, "{ind}    (subcmd)").unwrap();
        writeln!(out, "{ind}        local -a subcommands").unwrap();
        writeln!(out, "{ind}        subcommands=(").unwrap();
        for subcmd in cmd.commands.iter().filter(|s| !s.command.hidden) {
            let desc = subcmd.command.description.replace('\'', "'\\''").replace(':', "\\:");
            writeln!(out, "{ind}            '{}:{desc}'", subcmd.name).unwrap();
        }
        writeln!(out, "{ind}        )").unwrap();
        writeln!(out, "{ind}        if (( CURRENT == 1 )); then").unwrap();
        writeln!(out, "{ind}          _describe -t commands '{} commands' subcommands", cmd.name)
            .unwrap();
        writeln!(out, "{ind}            return").unwrap();
        writeln!(out, "{ind}        fi").unwrap();
        writeln!(out, "{ind}        local cmd=$words[1]").unwrap();
        writeln!(out, "{ind}        curcontext=\"${{curcontext%:*:*}}:{prefix}-$cmd\"").unwrap();
        writeln!(out, "{ind}        case $cmd in").unwrap();
        for subcmd in cmd.commands.iter().filter(|s| !s.command.hidden) {
            writeln!(out, "{ind}            ({})", subcmd.name).unwrap();
            writeln!(out, "{ind}                _{prefix}_{}", subcmd.name).unwrap();
            writeln!(out, "{ind}                ;;").unwrap();
        }
        writeln!(out, "{ind}        esac").unwrap();
        writeln!(out, "{ind}        ;;").unwrap();
        writeln!(out, "{ind}esac").unwrap();
    }
}
