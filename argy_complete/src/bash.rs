// Copyright (c) 2026 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Generation of completions for Bash.

use crate::Generator;
use argy_shared::{CommandInfoWithArgs, FlagInfoKind};
use std::fmt::Write;

/// A generator for Bash shell completions.
pub struct Bash;

impl Generator for Bash {
    // Escaped `{{...}}` produce bash `${...}` templates, not format args.
    #[allow(clippy::literal_string_with_formatting_args)]
    fn generate(cmd_name: &str, cmd: &CommandInfoWithArgs<'_>) -> String {
        let mut out = String::new();

        // Basic bash completion uses `complete -F _func cmd`
        // We'll generate a recursive function structure for subcommands.

        // The runtime cmd_name (argv[0] basename) is attacker-influenced; sanitize it to a safe
        // shell identifier before interpolating into the emitted script.
        let safe = crate::safe_cmd_name(cmd_name);

        writeln!(&mut out, "_{safe}() {{").unwrap();
        writeln!(&mut out, "    local i cur prev opts cmds").unwrap();
        writeln!(&mut out, "    COMPREPLY=()").unwrap();
        writeln!(&mut out, "    cur=\"${{COMP_WORDS[COMP_CWORD]}}\"").unwrap();
        writeln!(&mut out, "    if [[ $COMP_CWORD -ge 1 ]]; then").unwrap();
        writeln!(&mut out, "        prev=\"${{COMP_WORDS[COMP_CWORD-1]}}\"").unwrap();
        writeln!(&mut out, "    else").unwrap();
        writeln!(&mut out, "        prev=\"\"").unwrap();
        writeln!(&mut out, "    fi").unwrap();
        writeln!(&mut out, "    cmd=\"\"").unwrap();
        writeln!(&mut out, "    opts=\"\"").unwrap();
        writeln!(&mut out).unwrap();

        // Find the current command by traversing from the beginning
        writeln!(&mut out, "    for i in ${{COMP_WORDS[@]}}; do").unwrap();
        writeln!(&mut out, "        case \"${{i}}\" in").unwrap();
        writeln!(&mut out, "            {safe} | */{safe})").unwrap();
        writeln!(&mut out, "                cmd=\"{safe}\"").unwrap();
        writeln!(&mut out, "                ;;").unwrap();
        for subcmd in cmd.commands.iter().filter(|s| !s.command.hidden) {
            // Also need to handle nested subcommands, but bash scripts often hardcode
            // the full path like `cmd_subcmd` depending on how nested it is.
            // Let's keep it simple for a single level first or nested if needed.
            generate_bash_case(&mut out, &safe, &subcmd.command);
        }
        writeln!(&mut out, "        esac").unwrap();
        writeln!(&mut out, "    done").unwrap();
        writeln!(&mut out).unwrap();

        // Now dispatch based on the determined `cmd`
        writeln!(&mut out, "    case \"${{cmd}}\" in").unwrap();
        generate_all_dispatch(&mut out, &safe, cmd);
        writeln!(&mut out, "    esac").unwrap();
        writeln!(&mut out, "}}").unwrap();
        writeln!(&mut out).unwrap();
        writeln!(&mut out, "complete -F _{safe} -o bashdefault -o default {safe}").unwrap();

        out
    }
}

fn generate_bash_case(out: &mut String, prefix: &str, cmd: &CommandInfoWithArgs<'_>) {
    let name = crate::sanitize_ident(cmd.name);
    let full_name = format!("{prefix}_{name}");
    writeln!(out, "            {name})").unwrap();
    writeln!(out, "                cmd=\"{full_name}\"").unwrap();
    writeln!(out, "                ;;").unwrap();
    for subcmd in &cmd.commands {
        generate_bash_case(out, &full_name, &subcmd.command);
    }
}

// Escaped `{{...}}` produce bash `${...}` templates, not format args.
#[allow(clippy::literal_string_with_formatting_args)]
fn generate_bash_dispatch(out: &mut String, full_name: &str, cmd: &CommandInfoWithArgs<'_>) {
    writeln!(out, "        {full_name})").unwrap();

    let mut opts = Vec::new();
    for flag in cmd.flags {
        if !flag.long.is_empty() {
            opts.push(crate::sanitize_ident(flag.long));
        }
        if let Some(short) = flag.short {
            opts.push(format!("-{short}"));
        }
    }

    let mut cmds = Vec::new();
    for subcmd in cmd.commands.iter().filter(|s| !s.command.hidden) {
        cmds.push(crate::sanitize_ident(subcmd.name));
    }

    if !opts.is_empty() {
        writeln!(out, "            opts=\"{}\"", opts.join(" ")).unwrap();
    }
    if !cmds.is_empty() {
        writeln!(out, "            cmds=\"{}\"", cmds.join(" ")).unwrap();
    }

    if !opts.is_empty() || !cmds.is_empty() {
        writeln!(out, "            case \"${{prev}}\" in").unwrap();
        for flag in cmd.flags {
            if let FlagInfoKind::Option { .. } = flag.kind {
                let mut prev_matches = Vec::new();
                if !flag.long.is_empty() {
                    prev_matches.push(crate::sanitize_ident(flag.long));
                }
                if let Some(short) = flag.short {
                    prev_matches.push(format!("-{short}"));
                }
                if !prev_matches.is_empty() {
                    writeln!(out, "                {})", prev_matches.join(" | ")).unwrap();
                    writeln!(out, "                    COMPREPLY=()").unwrap();
                    writeln!(out, "                    return 0").unwrap();
                    writeln!(out, "                    ;;").unwrap();
                }
            }
        }
        writeln!(out, "                *)").unwrap();
        writeln!(out, "                    COMPREPLY=( $(compgen -W \"${{opts}} ${{cmds}}\" -- \"${{cur}}\") )").unwrap();
        writeln!(out, "                    return 0").unwrap();
        writeln!(out, "                    ;;").unwrap();
        writeln!(out, "            esac").unwrap();
    }

    writeln!(out, "            ;;").unwrap();
}
fn generate_all_dispatch(out: &mut String, full_name: &str, cmd: &CommandInfoWithArgs<'_>) {
    generate_bash_dispatch(out, full_name, cmd);
    for subcmd in cmd.commands.iter().filter(|s| !s.command.hidden) {
        let next_full_name = format!("{}_{}", full_name, subcmd.name);
        generate_all_dispatch(out, &next_full_name, &subcmd.command);
    }
}
