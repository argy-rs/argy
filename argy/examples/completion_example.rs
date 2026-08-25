// Copyright (c) 2026 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use argy::{ArgsInfo, FromArgs};
use argy_complete::Generator;

#[derive(FromArgs, ArgsInfo)]
/// An example command showing off autocompletion generation.
struct MyCmd {
    /// do things verbosely
    #[argy(switch, short = 'v')]
    verbose: bool,
    #[argy(subcommand)]
    cmd: Subcommands,
}

#[derive(FromArgs, ArgsInfo)]
#[argy(subcommand)]
enum Subcommands {
    Completion(CompletionCmd),
    DoThings(DoThingsCmd),
    DoMoreThings(DoMoreThingsCmd),
}

#[derive(FromArgs, ArgsInfo)]
/// Generate shell completions.
#[argy(subcommand, name = "completion")]
struct CompletionCmd {
    /// the shell to generate for (bash, zsh, fish, nushell)
    #[argy(positional)]
    shell: String,
}

#[derive(FromArgs, ArgsInfo)]
/// Do some things.
#[argy(subcommand, name = "do-things")]
struct DoThingsCmd {
    /// how many things to do
    #[argy(option, short = 'n', default = "5")]
    count: usize,

    /// do it quickly
    #[argy(switch, short = 'q')]
    quick: bool,
}

#[derive(FromArgs, ArgsInfo)]
#[argy(subcommand)]
enum MoreThingsSubcommands {
    ThingOne(ThingOneCommand),
    ThingTwo(ThingTwoCommand),
}

#[derive(FromArgs, ArgsInfo)]
/// Do thing one.
#[argy(subcommand, name = "one")]
struct ThingOneCommand {
    /// do it slowly
    #[argy(switch, short = 's')]
    slow: bool,
}

#[derive(FromArgs, ArgsInfo)]
/// Do thing two.
#[argy(subcommand, name = "two")]
struct ThingTwoCommand {
    /// do it quickly
    #[argy(switch, short = 'q')]
    quick: bool,
}

#[derive(FromArgs, ArgsInfo)]
/// Do some more things.
#[argy(subcommand, name = "do-more-things")]
struct DoMoreThingsCmd {
    #[argy(subcommand)]
    cmd: MoreThingsSubcommands,
}

fn main() {
    let args: MyCmd = argy::from_env();

    if args.verbose && matches!(args.cmd, Subcommands::Completion(_)) {
        println!("Doing things verbosely ");
    }

    match args.cmd {
        Subcommands::Completion(cmd) => {
            let cmd_info = MyCmd::get_args_info();
            let mut command_name = std::env::args().next().map_or_else(String::new, |arg0| {
                std::path::Path::new(&arg0)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string()
            });
            if command_name.is_empty() {
                command_name = cmd_info.name.to_string();
            }
            match cmd.shell.as_str() {
                "bash" => {
                    println!("{}", argy_complete::bash::Bash::generate(&command_name, &cmd_info));
                }
                "zsh" => {
                    println!("{}", argy_complete::zsh::Zsh::generate(&command_name, &cmd_info));
                }
                "fish" => {
                    println!("{}", argy_complete::fish::Fish::generate(&command_name, &cmd_info));
                }
                "nushell" => {
                    println!(
                        "{}",
                        argy_complete::nushell::Nushell::generate(&command_name, &cmd_info)
                    );
                }
                _ => eprintln!("Unsupported shell: {}", cmd.shell),
            }
        }
        Subcommands::DoThings(cmd) => {
            println!("Doing {} things (quick: {})", cmd.count, cmd.quick);
        }
        Subcommands::DoMoreThings(cmd) => match cmd.cmd {
            MoreThingsSubcommands::ThingOne(cmd) => {
                println!("Doing more thing one (slow: {})", cmd.slow);
            }
            MoreThingsSubcommands::ThingTwo(cmd) => {
                println!("Doing more thing two (quick: {})", cmd.quick);
            }
        },
    }
}
