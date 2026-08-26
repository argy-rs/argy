# Argy
**Argy is an opinionated Derive-based argument parser optimized for code size**

[![crates.io](https://img.shields.io/crates/v/argy.svg)](https://crates.io/crates/argy)
[![license](https://img.shields.io/badge/license-BSD3.0-blue.svg)](https://github.com/argy-rs/argy/LICENSE)
[![docs.rs](https://docs.rs/argy/badge.svg)](https://docs.rs/crate/argy/)
![Argy](https://github.com/argy-rs/argy/workflows/Argy/badge.svg)

Derive-based argument parsing optimized for code size and conformance
to the Fuchsia commandline tools specification

The public API of this library consists primarily of the `FromArgs`
derive and the `from_env` function, which can be used to produce
a top-level `FromArgs` type from the current program's commandline
arguments.

## Basic Example

```rust,no_run
use argy::FromArgs;

#[derive(FromArgs)]
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

fn main() {
    let up: GoUp = argy::from_env();
}
```

`./some_bin --help` will then output the following:

```
Usage: cmdname [-j] --height <height> [--pilot-nickname <pilot-nickname>]

Reach new heights.

Options:
  -j, --jump        whether or not to jump
  --height          how high to go
  --pilot-nickname  an optional nickname for the pilot
  --help, help      display usage information
```

The resulting program can then be used in any of these ways:
- `./some_bin --height 5`
- `./some_bin -j --height 5`
- `./some_bin --jump --height 5 --pilot-nickname Wes`

Switches, like `jump`, are optional and will be set to true if provided.

Options, like `height` and `pilot_nickname`, can be either required,
optional, or repeating, depending on whether they are contained in an
`Option` or a `Vec`. Default values can be provided using the
`#[argy(default = "<your_code_here>")]` attribute, and in this case an
option is treated as optional.

```rust
use argy::FromArgs;

fn default_height() -> usize {
    5
}

#[derive(FromArgs)]
/// Reach new heights.
struct GoUp {
    /// an optional nickname for the pilot
    #[argy(option)]
    pilot_nickname: Option<String>,

    /// an optional height
    #[argy(option, default = "default_height()")]
    height: usize,

    /// an optional direction which is "up" by default
    #[argy(option, default = "String::from(\"only up\")")]
    direction: String,
}

fn main() {
    let up: GoUp = argy::from_env();
}
```

Options and switches can also be sourced from environment variables using the
`env` attribute. The environment variable only supplies the value when the
option or switch is not provided on the command line, so a CLI value always
takes precedence. An `env`-sourced value still counts as providing a required
option.

```rust
use argy::FromArgs;

#[derive(FromArgs)]
/// Reach new heights.
struct GoUp {
    /// an optional height, falling back to the `HEIGHT` env var
    #[argy(option, env = "HEIGHT")]
    height: Option<usize>,

    /// whether to jump, reading the `JUMP` env var when `--jump` is absent
    #[argy(switch, env = "JUMP")]
    jump: bool,
}

fn main() {
    let up: GoUp = argy::from_env();
}
```

Custom option types can be deserialized so long as they implement the
`FromArgValue` trait (automatically implemented for all `FromStr` types).
If more customized parsing is required, you can supply a custom
`fn(&str) -> Result<T, String>` using the `from_str_fn` attribute:

```rust
use argy::FromArgs;

#[derive(FromArgs)]
/// Goofy thing.
struct FiveStruct {
    /// always five
    #[argy(option, from_str_fn(always_five))]
    five: usize,
}

fn always_five(_value: &str) -> Result<usize, String> {
    Ok(5)
}
```

Positional arguments can be declared using `#[argy(positional)]`.
These arguments will be parsed in order of their declaration in
the structure:

```rust
use argy::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// A command with positional arguments.
struct WithPositional {
    #[argy(positional)]
    first: String,
}
```

The last positional argument may include a default, or be wrapped in
`Option` or `Vec` to indicate an optional or repeating positional argument.

Subcommands are also supported. To use a subcommand, declare a separate
`FromArgs` type for each subcommand as well as an enum that cases
over each command:

```rust
use argy::FromArgs;

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
#[argy(subcommand, name = "one")]
struct SubCommandOne {
    #[argy(option)]
    /// how many x
    x: usize,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Second subcommand.
#[argy(subcommand, name = "two", short = 't')]
struct SubCommandTwo {
    #[argy(switch)]
    /// whether to fooey
    fooey: bool,
}
```

## Advanced Description

You can define a complex help output that includes an **Examples** section.
Use a `{command_name}` placeholder.

```rust
#[derive(FromArgs, Debug)]
#[argy(
    description = "{command_name} is a tool to reach new heights.\n\n\
    Start exploring new heights:\n\n\
    \u{00A0} {command_name} --height 5 jump\n\
    ",
    example = "\
    {command_name} --height 5\n\
    {command_name} --height 5 j\n\
    {command_name} --height 5 --pilot-nickname Wes jump"
)]
pub struct CliArgs {
    /// how high to go
    #[argy(option)]
    height: usize,
    /// an optional nickname for the pilot
    #[argy(option)]
    pilot_nickname: Option<String>,
    /// command to execute
    #[argy(subcommand)]
    pub command: Command,
}
```

Output:

```
Usage: goup --height <height> [--pilot-nickname <pilot-nickname>] <command> [<args>]

goup is a tool to reach new heights.

Start exploring new heights:

  goup --height 5 jump

Options:
  --height          how high to go
  --pilot-nickname  an optional nickname for the pilot
  --help, help      display usage information

Commands:
  jump  j           whether or not to jump

Examples:
  goup --height 5
  goup --height 5 j
  goup --height 5 --pilot-nickname Wes jump
```

## How to debug the expanded derive macro for `argy`

The `argy::FromArgs` derive macro can be debugged with the [cargo-expand](https://crates.io/crates/cargo-expand) crate.

### Expand the derive macro in `examples/simple_example.rs`

See [argy/examples/simple_example.rs](./argy/examples/simple_example.rs) for the example struct we wish to expand.

First, install `cargo-expand` by running `cargo install cargo-expand`. Note this requires the nightly build of Rust.

Once installed, run `cargo expand` with in the `argy` package and you can see the expanded code.

## Note

This is not an officially supported Google product.
