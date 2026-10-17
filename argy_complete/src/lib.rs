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

#[cfg(test)]
mod tests;
