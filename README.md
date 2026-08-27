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

## Optional-value options

An option declared with `optional_value` may be provided either bare (with no
value) or with an explicit value, matching clap's `num_args=0..=1`. When it
appears bare, it is filled with the value given by `default_missing_value`;
when provided with a value (via `--flag=value` or `--flag value`) that value
is used instead.

```rust
use argy::FromArgs;

#[derive(FromArgs)]
/// Has an optional-value option.
struct GoUp {
    /// a height that may be given explicitly or default to 5 when bare
    #[argy(option, optional_value, default_missing_value = "5")]
    height: usize,

    /// an optional value that may be bare or explicit
    #[argy(option, optional_value, default_missing_value = "/tmp/out")]
    output: Option<std::path::PathBuf>,
}

fn main() {
    let up: GoUp = argy::from_env();
    // `--height 7`      => height: 7
    // `--height=7`      => height: 7
    // `--height`        => height: 5 (the `default_missing_value`)
    // omitted           => error: height is required (no default)
    println!("height: {}, output: {:?}", up.height, up.output);
}
```

An `optional_value` option behaves like any other option with respect to
requiredness: a plain field (not `Option` and with no `default`) is required
and errors when omitted, while an `Option<T>` field is `None` when absent and
a field with `default` falls back to that value when absent. The
`default_missing_value` is only consulted when the option is actually provided
without a value.

## Requires

An option or switch may require that one or more other options or switches
also be present whenever it is provided. A violation is a usage error (the
same exit-code-2 path as a missing required option) that lists the missing
required option and prints usage. Use the single form `requires = "other"` to
name one required option, or the list form `requires = ["a", "b"]` to name
several. `requires` is valid on `#[argy(option)]` and `#[argy(switch)]` fields,
and each referenced name must be an existing option/switch long name.

```rust
use argy::FromArgs;

#[derive(FromArgs)]
/// Connect to a remote host.
struct Connect {
    /// target host
    #[argy(option, requires = "user")]
    host: Option<String>,

    /// user name
    #[argy(option)]
    user: Option<String>,
}

fn main() {
    let c: Connect = argy::from_env();
}
```

Providing `--host <h>` without `--user <u>` fails with a usage error; providing
both (or neither) succeeds. Requirements may be mutual: if `a` requires `b`
and `b` requires `a`, then either alone fails and both together succeed.

## Value enums

## Value enums

## Value enums

Fieldless enums can derive `argy::ValueEnum` to get standalone parsing and
rendering without a manual `FromStr` impl. The derive implements
`std::str::FromStr`, `std::fmt::Display`, the `argy::ValueEnum` trait
(`value_variants()` / `to_possible_value()`), and — via the blanket `FromStr`
impl — `argy::FromArgValue`, so a `ValueEnum` can be used directly as an
`#[argy(option)]` value.

Variant names are converted to kebab-case by default (matching clap). Use
`#[argy(rename_all = "snake_case")]` for snake-case, and override individual
values with `name` and `alias`:

```rust
use argy::{FromArgs, ValueEnum};

#[derive(ValueEnum, Debug, PartialEq)]
#[argy(rename_all = "snake_case")]
enum Mode {
    SoftCore,
    HardCore,
}

#[derive(FromArgs)]
/// Do the thing.
struct DoIt {
    /// how to do it.
    #[argy(option)]
    mode: Mode,
}

fn main() {
    let cmd: DoIt = argy::from_env();
    // `--mode soft_core` parses to Mode::SoftCore
    println!("mode: {}", cmd.mode);
}
```

For kebab-case (the default), `HardCore` becomes `hard-core`; an explicit
`#[argy(name = "...", alias = "...")]` on a variant overrides its canonical
name and adds accepted aliases.

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

If that final positional argument is wrapped in `Vec` and marked with the
`last` attribute, it acts as a trailing variadic that consumes only the
trailing arguments after all options have been parsed. Options may appear
before or after its values and are still parsed as options (unlike the
greedy capture of all remaining input). Its usage line renders with a `--`
separator, mirroring clap's `last = true`:

```rust
use argy::FromArgs;

#[derive(FromArgs, PartialEq, Debug)]
/// A command with a trailing variadic positional.
struct Run {
    /// how verbose
    #[argy(switch)]
    verbose: bool,
    /// command and its arguments
    #[argy(positional, last)]
    command: Vec<String>,
}
```

`["foo", "bar"]` (the `--` separates the trailing positional); `foo bar`
parses `command` as `["foo", "bar"]`.
`["foo", "--", "bar"]`; `foo bar` parses `command` as `["foo", "bar"]`.
Only the last positional may be marked `last`, and it must be a `Vec`.

Subcommands are also supported. To use a subcommand, declare a separate
`FromArgs` type for each subcommand as well as an enum that cases
over each command:

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

## Command-tree introspection

`#[derive(ArgsInfo)]` (alongside `#[derive(FromArgs)]`) exposes the parsed
command structure at runtime, so an app can render a reference or feed shell
completion generation without parsing `--help` output. It implements the
`argy::ArgsInfo` trait:

```rust,no_run
use argy::ArgsInfo;

let tree: argy::CommandInfoWithArgs = MyCommand::get_args_info();
```

`get_args_info()` returns an `argy::CommandInfoWithArgs` — the same
`argy_shared` type consumed by the completion generators — carrying the
command `name`, `description` (the "about" text), `aliases`, `flags` (options
and switches), `positionals`, and nested `commands`. Each subcommand is a
`SubCommandInfo` whose `command` is itself a `CommandInfoWithArgs`, so the
tree is fully nested and can be walked recursively to render a reference:

```rust,no_run
use argy::{ArgsInfo, CommandInfoWithArgs};
use std::fmt::Write;

fn render_reference(out: &mut String, cmd: &CommandInfoWithArgs) {
    writeln!(out, "# {}", cmd.name).unwrap();
    for alias in cmd.aliases {
        writeln!(out, "  alias: {alias}").unwrap();
    }
    for flag in cmd.flags {
        let mut line = String::from(flag.long);
        for alias in flag.aliases {
            write!(line, " / {alias}").unwrap();
        }
        writeln!(out, "  flag: {line}").unwrap();
    }
    for sub in &cmd.commands {
        render_reference(out, &sub.command);
    }
}
```

The introspection covers every element of the command tree: options
(`FlagInfoKind::Option`), switches (`FlagInfoKind::Switch`), positionals
(`PositionalInfo`), nested subcommands, and aliases — both command aliases (on
`CommandInfoWithArgs::aliases`, the alternative names accepted for a
subcommand) and flag aliases (on `FlagInfo::aliases`, given as
`--`-prefixed long names).

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
