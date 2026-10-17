// Copyright (c) 2026 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Autocompletion generators for `argy`-based CLIs.

pub mod bash;
pub mod fish;
pub mod nushell;
pub mod zsh;

use argy_shared::CommandInfoWithArgs;

/// Escape a description so it cannot break out of the generated-script context
/// it is interpolated into. Newlines (and carriage returns) would terminate a
/// `# comment` or a single-quoted string and inject a new statement, so they
/// are collapsed to a space. Backslashes and single quotes are escaped so
/// single-quoted contexts (fish) cannot be broken out of.
pub(crate) fn escape_description(desc: &str) -> String {
    desc.replace(['\r', '\n'], " ").replace('\\', "\\\\").replace('\'', "\\'")
}

/// A trait for generating shell completions.
pub trait Generator {
    /// Generates the completion script for the given command name and structure.
    fn generate(cmd_name: &str, cmd: &CommandInfoWithArgs<'_>) -> String;
}

/// Characters that shell syntax can interpret and that must never be interpolated raw into a
/// generated script (whitespace plus shell metacharacters).
const fn is_shell_meta(c: char) -> bool {
    matches!(
        c,
        ' ' | '\t'
            | '\n'
            | '\r'
            | '$'
            | '`'
            | '"'
            | '\''
            | '\\'
            | ';'
            | '&'
            | '|'
            | '('
            | ')'
            | '<'
            | '>'
            | '*'
            | '?'
            | '['
            | ']'
            | '{'
            | '}'
            | '!'
            | '#'
            | '~'
    )
}

/// Return a shell-safe identifier derived from a runtime command name (the `argv[0]` basename).
///
/// `cmd_name` is attacker-influenced, so it is never safe to interpolate verbatim. Every
/// character that is not ASCII alphanumeric or `_` is replaced with `_`, and a leading digit is
/// prefixed so the result is always a valid shell identifier/function name that contains no shell
/// metacharacters. Legitimate names like `mycmd` are returned unchanged.
pub(crate) fn safe_cmd_name(name: &str) -> String {
    let mut out: String =
        name.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if out.is_empty() {
        out.push('a');
    }
    if out.as_bytes()[0].is_ascii_digit() {
        out.insert(0, '_');
    }
    out
}

/// Strip whitespace and shell metacharacters from an identifier (option/subcommand/positional
/// name) so it cannot break out of the emitted context. No-op for ordinary identifiers like
/// `--verbose` or `do-more-things`.
pub(crate) fn sanitize_ident(value: &str) -> String {
    value.chars().filter(|c| !is_shell_meta(*c)).collect()
}

#[cfg(test)]
mod tests;
