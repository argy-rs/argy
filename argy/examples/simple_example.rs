// Copyright (c) 2022 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use {argy::FromArgs, std::fmt::Debug};

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
}

#[derive(FromArgs, PartialEq, Debug)]
/// First subcommand.
#[argy(subcommand, name = "one", short = 'o')]
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

    #[argy(option)]
    /// how to woot
    woot: Woot,
}

#[derive(argy::FromArgValue, PartialEq, Debug)]
enum Woot {
    Quiet,
    Loud,
}

fn main() {
    let toplevel: TopLevel = argy::from_env();
    println!("{toplevel:#?}");
}
