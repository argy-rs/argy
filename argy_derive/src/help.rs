// Copyright (c) 2020 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use std::fmt::Write;
use {
    crate::{
        errors::Errors,
        parse_attrs::{Description, FieldKind, TypeAttrs},
        ty_is_option_bool, Optionality, StructField,
    },
    argy_shared::{DESCRIPTION_PADDING, INDENT},
    proc_macro2::{Span, TokenStream},
    quote::quote,
};

const SECTION_SEPARATOR: &str = "\n\n";

/// The column at which a description should start so that a name of
/// `max_name_len` characters is followed by a [`DESCRIPTION_PADDING`]-space
/// separator, matching the longest name in the group.
fn indent_for(max_name_len: usize) -> usize {
    max_name_len + INDENT.chars().count() + DESCRIPTION_PADDING
}

/// Returns a `TokenStream` generating a `String` help message.
///
/// Note: `fields` entries with `is_subcommand.is_some()` will be ignored
#[allow(clippy::too_many_lines)] // Help assembly is one cohesive routine; splitting would harm readability.
#[allow(clippy::literal_string_with_formatting_args)] // `{metadata}` is a deliberate runtime `format!` placeholder.
#[allow(clippy::needless_pass_by_value)] // Signature matches lib.rs callers that pass the ident by value.
pub fn help(
    errors: &Errors,
    cmd_name_str_array_ident: syn::Ident,
    ty_attrs: &TypeAttrs,
    fields: &[StructField<'_>],
    subcommand: Option<&StructField<'_>>,
    help_triggers: &[String],
) -> TokenStream {
    let mut format_lit = "Usage: {command_name}".to_string();

    let positional = fields.iter().filter(|f| {
        f.kind == FieldKind::Positional && f.attrs.greedy.is_none() && !f.attrs.hidden_help
    });
    let has_positional = positional.clone().next().is_some();
    let options = fields.iter().filter(|f| f.long_name.is_some() && !f.attrs.hidden_help);

    if let Some(usage) = &ty_attrs.usage {
        format_lit.push(' ');
        format_lit.push_str(&usage.value());
    } else {
        let has_explicit_usage = std::iter::empty()
            .chain(positional.clone())
            .chain(options.clone())
            .any(|p| p.attrs.usage);
        let positional = positional.clone().filter(|p| !has_explicit_usage || p.attrs.usage);
        let options = options.clone().filter(|p| !has_explicit_usage || p.attrs.usage);

        for option in options.clone() {
            format_lit.push(' ');
            option_usage(&mut format_lit, option);
        }

        if has_positional && subcommand.is_none() {
            format_lit.push_str(" [--]");
        }

        for arg in positional.clone() {
            format_lit.push(' ');
            positional_usage(&mut format_lit, arg);
        }

        let remain = fields.iter().filter(|f| {
            f.kind == FieldKind::Positional && f.attrs.greedy.is_some() && !f.attrs.hidden_help
        });
        for arg in remain {
            format_lit.push(' ');
            positional_usage(&mut format_lit, arg);
        }

        if let Some(subcommand) = subcommand {
            format_lit.push(' ');
            if !subcommand.optionality.is_required() {
                format_lit.push('[');
            }
            format_lit.push_str("<command>");
            if !subcommand.optionality.is_required() {
                format_lit.push(']');
            }
            format_lit.push_str(" [<args>]");
        }
    }

    format_lit.push_str(SECTION_SEPARATOR);

    let description = require_description(errors, Span::call_site(), &ty_attrs.description, "type");
    format_lit.push_str(&description);

    // Render `repository`/`homepage` from Cargo.toml metadata into help when the
    // corresponding `#[argy(...)]` attribute is present. The `{metadata}`
    // placeholder is filled at runtime with the non-empty `CARGO_PKG_*` values.
    format_lit.push_str("{metadata}");

    if has_positional {
        format_lit.push_str(SECTION_SEPARATOR);
        format_lit.push_str("Positional Arguments:");
        let positional_indent = indent_for(
            positional.clone().map(|p| p.positional_arg_name().chars().count()).max().unwrap_or(0),
        );
        for arg in positional {
            positional_description(&mut format_lit, arg, positional_indent);
        }
    }

    format_lit.push_str(SECTION_SEPARATOR);
    format_lit.push_str("Options:");
    let option_indent = indent_for(
        options
            .clone()
            .map(|o| {
                let long = o.long_name.as_ref().expect("missing long name for option");
                option_name(o.attrs.short.as_ref().map(syn::LitChar::value), long).chars().count()
            })
            .chain(std::iter::once(help_triggers.join(", ").chars().count()))
            .max()
            .unwrap_or(0),
    );
    for option in options {
        option_description(errors, &mut format_lit, option, option_indent);
    }
    option_description_format(
        &mut format_lit,
        None,
        &help_triggers.join(", "),
        "display usage information",
        option_indent,
    );

    let subcommand_calculation;
    let subcommand_format_arg;
    if let Some(subcommand) = subcommand {
        format_lit.push_str(SECTION_SEPARATOR);
        format_lit.push_str("Commands:{subcommands}");
        let subcommand_ty = subcommand.ty_without_wrapper;
        subcommand_format_arg = quote! { subcommands = subcommands };
        subcommand_calculation = quote! {
            let subcommands = argy::print_subcommands(
                <#subcommand_ty as argy::SubCommands>::COMMANDS
                    .iter()
                    .copied()
                    .chain(
                        <#subcommand_ty as argy::SubCommands>::dynamic_commands()
                            .iter()
                            .copied())
            );
        };
    } else {
        subcommand_calculation = TokenStream::new();
        subcommand_format_arg = TokenStream::new();
    }

    lits_section(&mut format_lit, "Examples:", &ty_attrs.examples);

    lits_section(&mut format_lit, "Notes:", &ty_attrs.notes);

    if !ty_attrs.error_codes.is_empty() {
        format_lit.push_str(SECTION_SEPARATOR);
        format_lit.push_str("Error codes:");
        for (code, text) in &ty_attrs.error_codes {
            format_lit.push('\n');
            format_lit.push_str(INDENT);
            write!(format_lit, "{} {}", code, text.value()).unwrap();
        }
    }

    format_lit.push('\n');

    // Build runtime statements that fill `{metadata}` from `CARGO_PKG_REPOSITORY`
    // and `CARGO_PKG_HOMEPAGE`, but only for the attributes that are present and
    // only when the corresponding Cargo.toml field is non-empty.
    let mut metadata_stmts = Vec::new();
    for (present, env, label) in [
        (ty_attrs.repository.is_some(), "CARGO_PKG_REPOSITORY", "Repository:"),
        (ty_attrs.homepage.is_some(), "CARGO_PKG_HOMEPAGE", "Homepage:"),
        (ty_attrs.author.is_some(), "CARGO_PKG_AUTHORS", "Author:"),
    ] {
        if present {
            metadata_stmts.push(quote! {
                if let Some(v) = ::core::option_env!(#env) {
                    if !v.is_empty() {
                        if __first_metadata {
                            s.push_str("\n\n");
                            __first_metadata = false;
                        } else {
                            s.push('\n');
                        }
                        s.push_str(#label);
                        s.push(' ');
                        s.push_str(v);
                    }
                }
            });
        }
    }

    let metadata = quote! {
        let __metadata = {
            let mut s = ::std::string::String::new();
            let mut __first_metadata = true;
            #(#metadata_stmts)*
            s
        };
    };

    quote! { {
        #subcommand_calculation
        #metadata
        format!(#format_lit, metadata = __metadata, command_name = #cmd_name_str_array_ident.join(" "), #subcommand_format_arg)
    } }
}

/// A section composed of exactly just the literals provided to the program.
fn lits_section(out: &mut String, heading: &str, lits: &[syn::LitStr]) {
    if !lits.is_empty() {
        out.push_str(SECTION_SEPARATOR);
        out.push_str(heading);
        for lit in lits {
            let value = lit.value();
            for line in value.split('\n') {
                out.push('\n');
                out.push_str(INDENT);
                out.push_str(line);
            }
        }
    }
}

/// Add positional arguments like `[<foo>...]` to a help format string.
fn positional_usage(out: &mut String, field: &StructField<'_>) {
    let required =
        field.optionality.is_required() || (field.attrs.greedy.is_some() && field.attrs.required);
    if !required {
        out.push('[');
    }
    if field.attrs.greedy.is_none() {
        out.push('<');
    }
    let name = field.positional_arg_name();
    out.push_str(&name);
    if matches!(field.optionality, Optionality::Repeating | Optionality::DefaultedRepeating(_)) {
        out.push_str("...");
    }
    if field.attrs.greedy.is_none() {
        out.push('>');
    }
    if !required {
        out.push(']');
    }
}

/// Add options like `[-f <foo>]` to a help format string.
/// This function must only be called on options (things with `long_name.is_some()`)
fn option_usage(out: &mut String, field: &StructField<'_>) {
    // bookend with `[` and `]` if optional
    if !field.optionality.is_required() {
        out.push('[');
    }

    let long_name = field.long_name.as_ref().expect("missing long name for option");
    if let Some(short) = field.attrs.short.as_ref() {
        out.push('-');
        out.push(short.value());
    } else {
        out.push_str(long_name);
    }

    match field.kind {
        FieldKind::SubCommand | FieldKind::Positional => unreachable!(), // don't have long_name
        FieldKind::Switch => {
            // An `Option<bool>` switch accepts an optional inline value,
            // so render it as `[--flag[=<bool>]]`.
            if ty_is_option_bool(&field.field.ty) {
                out.push_str("[=<bool>]");
            }
        }
        FieldKind::Option => {
            if field.attrs.optional_value {
                // An optional-value option accepts a bare occurrence or an
                // explicit value, so render the value part as `[=<name>]`.
                out.push_str("[=");
                if let Some(arg_name) = &field.attrs.arg_name {
                    out.push_str(&arg_name.value());
                } else {
                    out.push_str(long_name.trim_start_matches("--"));
                }
                out.push(']');
            } else {
                out.push_str(" <");
                if let Some(arg_name) = &field.attrs.arg_name {
                    out.push_str(&arg_name.value());
                } else {
                    out.push_str(long_name.trim_start_matches("--"));
                }
                if matches!(
                    field.optionality,
                    Optionality::Repeating | Optionality::DefaultedRepeating(_)
                ) {
                    out.push_str("...");
                }
                out.push('>');
            }
        }
    }

    if !field.optionality.is_required() {
        out.push(']');
    }
}

#[allow(clippy::ref_option)] // Signature shared with lib.rs and args_info.rs callers passing `&Option`.
pub fn require_description(
    errors: &Errors,
    err_span: Span,
    desc: &Option<Description>,
    kind: &str, // the thing being described ("type" or "field"),
) -> String {
    desc.as_ref().map_or_else(
        || {
            errors.err_span(
                err_span,
                &format!(
                    "#[derive(FromArgs)] {kind} with no description.
Add a doc comment or an `#[argy(description = \"...\")]` attribute."
                ),
            );
            String::new()
        },
        |d| d.content.value().trim().to_owned(),
    )
}

/// Describes a positional argument like this:
///  hello       positional argument description
fn positional_description(out: &mut String, field: &StructField<'_>, description_indent: usize) {
    let field_name = field.positional_arg_name();

    let description = field
        .attrs
        .description
        .as_ref()
        .map(|d| d.content.value().trim().to_owned())
        .unwrap_or_default();
    positional_description_format(out, &field_name, &description, description_indent);
}

fn positional_description_format(
    out: &mut String,
    name: &str,
    description: &str,
    description_indent: usize,
) {
    let info =
        argy_shared::CommandInfo { name, description, short: &'\0', aliases: &[], hidden: false };
    argy_shared::write_description(out, &info, description_indent);
}

/// Describes an option like this:
///  -f, --force       force, ignore minor errors. This description
///                    is so long that it wraps to the next line.
fn option_description(
    errors: &Errors,
    out: &mut String,
    field: &StructField<'_>,
    description_indent: usize,
) {
    let short = field.attrs.short.as_ref().map(syn::LitChar::value);
    let long_with_leading_dashes = field.long_name.as_ref().expect("missing long name for option");
    let description =
        require_description(errors, field.name.span(), &field.attrs.description, "field");

    option_description_format(
        out,
        short,
        long_with_leading_dashes,
        &description,
        description_indent,
    );
}

/// Builds the displayed name for an option, e.g. `-f, --force` (short first)
/// or just `--force` when no short form exists.
fn option_name(short: Option<char>, long_with_leading_dashes: &str) -> String {
    let mut name = String::new();
    if let Some(short) = short {
        name.push('-');
        name.push(short);
        name.push_str(", ");
    }
    name.push_str(long_with_leading_dashes);
    name
}

fn option_description_format(
    out: &mut String,
    short: Option<char>,
    long_with_leading_dashes: &str,
    description: &str,
    description_indent: usize,
) {
    let name = option_name(short, long_with_leading_dashes);

    let info = argy_shared::CommandInfo {
        name: &name,
        description,
        short: &'\0',
        aliases: &[],
        hidden: false,
    };
    argy_shared::write_description(out, &info, description_indent);
}
