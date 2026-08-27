// Copyright (c) 2020 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use syn::{parse::Parser, punctuated::Punctuated, spanned::Spanned};

use {
    crate::errors::Errors,
    proc_macro2::Span,
    std::collections::hash_map::{Entry, HashMap},
};

/// Attributes applied to a field of a `#![derive(FromArgs)]` struct.
#[derive(Default)]
// Allow: many boolean config flags on this struct; refactoring would hurt readability.
#[allow(clippy::struct_excessive_bools)]
pub struct FieldAttrs {
    pub default: Option<syn::LitStr>,
    pub description: Option<Description>,
    pub from_str_fn: Option<syn::ExprPath>,
    pub field_type: Option<FieldType>,
    pub long: Option<syn::LitStr>,
    pub short: Option<syn::LitChar>,
    pub arg_name: Option<syn::LitStr>,
    pub greedy: Option<syn::Path>,
    /// Environment variable that supplies the value when the option/switch is
    /// not provided on the command line. Only valid on `#[argy(option)]` and
    /// `#[argy(switch)]` fields.
    pub env: Option<syn::LitStr>,
    /// Whether the option may appear bare (without a value), falling back to
    /// `default_missing_value`. Only valid on `#[argy(option)]` fields.
    pub optional_value: bool,
    /// The value used when an `optional_value` option is provided without an
    /// explicit value. Requires `optional_value`.
    pub default_missing_value: Option<syn::LitStr>,
    /// Whether a greedy positional must be provided at least once.
    /// Only valid on `#[argy(positional, greedy)]` fields.
    pub required: bool,
    pub hidden_help: bool,
    pub usage: bool,
    /// Whether the option or switch is global, i.e. also accepted after a
    /// subcommand is parsed. Only valid on `#[argy(option)]` and `#[argy(switch)]`.
    pub global: bool,
    /// Alternative long names for an option or switch.
    pub aliases: Vec<syn::LitStr>,
    /// Long names of other options/switches with which this one is mutually
    /// exclusive. Passing both is a parse error. Only valid on
    /// `#[argy(option)]` and `#[argy(switch)]` fields.
    pub conflicts_with: Vec<syn::LitStr>,
}

/// The purpose of a particular field on a `#![derive(FromArgs)]` struct.
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum FieldKind {
    /// Switches are booleans that are set to "true" by passing the flag.
    Switch,
    /// Options are `--key value`. They may be optional (using `Option`),
    /// or repeating (using `Vec`), or required (neither `Option` nor `Vec`)
    Option,
    /// Subcommand fields (of which there can be at most one) refer to enums
    /// containing one of several potential subcommands. They may be optional
    /// (using `Option`) or required (no `Option`).
    SubCommand,
    /// Positional arguments are parsed literally if the input
    /// does not begin with `-` or `--` and is not a subcommand.
    /// They are parsed in declaration order, and only the last positional
    /// argument in a type may be an `Option`, `Vec`, or have a default value.
    Positional,
}

/// The type of a field on a `#![derive(FromArgs)]` struct.
///
/// This is a simple wrapper around `FieldKind` which includes the `syn::Ident`
/// of the attribute containing the field kind.
pub struct FieldType {
    pub kind: FieldKind,
    pub ident: syn::Ident,
}

/// A description of a `#![derive(FromArgs)]` struct.
///
/// Defaults to the docstring if one is present, or `#[argy(description = "...")]`
/// if one is provided.
pub struct Description {
    /// Whether the description was an explicit annotation or whether it was a doc string.
    pub explicit: bool,
    pub content: syn::LitStr,
}

impl FieldAttrs {
    // Allow: this parse fn must handle every field-level argy attribute.
    #[allow(clippy::too_many_lines)]
    pub fn parse(errors: &Errors, field: &syn::Field) -> Self {
        let mut this = Self::default();
        let mut global_span = None;
        let mut required_span = None;

        for attr in &field.attrs {
            if is_doc_attr(attr) {
                parse_attr_doc(errors, attr, &mut this.description);
                continue;
            }

            let Some(ml) = argy_attr_to_meta_list(errors, attr) else {
                continue;
            };

            for meta in ml {
                let name = meta.path();
                if name.is_ident("alias") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_multi_string(errors, m, &mut this.aliases);
                    }
                } else if name.is_ident("conflicts_with") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_multi_string(errors, m, &mut this.conflicts_with);
                    }
                } else if name.is_ident("arg_name") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_arg_name(errors, m);
                    }
                } else if name.is_ident("default") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_default(errors, m);
                    }
                } else if name.is_ident("description") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_description(errors, m, &mut this.description);
                    }
                } else if name.is_ident("optional_value") {
                    if errors.expect_meta_word(&meta).is_some() {
                        this.optional_value = true;
                    }
                } else if name.is_ident("default_missing_value") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_single_string(
                            errors,
                            m,
                            "default_missing_value",
                            &mut this.default_missing_value,
                        );
                    }
                } else if name.is_ident("env") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_env(errors, m);
                    }
                } else if name.is_ident("from_str_fn") {
                    if let Some(m) = errors.expect_meta_list(&meta) {
                        this.parse_attr_from_str_fn(errors, m);
                    }
                } else if name.is_ident("long") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_long(errors, m);
                    }
                } else if name.is_ident("option") {
                    parse_attr_field_type(errors, &meta, FieldKind::Option, &mut this.field_type);
                } else if name.is_ident("short") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_short(errors, m);
                    }
                } else if name.is_ident("subcommand") {
                    parse_attr_field_type(
                        errors,
                        &meta,
                        FieldKind::SubCommand,
                        &mut this.field_type,
                    );
                } else if name.is_ident("switch") {
                    parse_attr_field_type(errors, &meta, FieldKind::Switch, &mut this.field_type);
                } else if name.is_ident("positional") {
                    parse_attr_field_type(
                        errors,
                        &meta,
                        FieldKind::Positional,
                        &mut this.field_type,
                    );
                } else if name.is_ident("greedy") {
                    this.greedy = Some(name.clone());
                } else if name.is_ident("required") {
                    this.required = true;
                    required_span = Some(meta.span());
                } else if name.is_ident("global") {
                    this.global = true;
                    global_span = Some(meta.span());
                } else if name.is_ident("hidden_help") {
                    this.hidden_help = true;
                } else if name.is_ident("usage") {
                    this.usage = true;
                } else {
                    errors.err(
                        &meta,
                        concat!(
                            "Invalid field-level `argy` attribute\n",
                            "Expected one of: `alias`, `arg_name`, `conflicts_with`, `default`, `default_missing_value`, `description`, `env`, `from_str_fn`, `global`, ",
                            "`greedy`, `long`, `option`, `optional_value`, `required`, `short`, `subcommand`, `switch`, `hidden_help`, `usage`",
                        ),
                    );
                }
            }
        }

        if this.optional_value {
            match this.field_type.as_ref().map(|f| f.kind) {
                Some(FieldKind::Option) => {}
                _ => {
                    errors.err(
                        field,
                        "`optional_value` may only be specified on `#[argy(option)]` fields",
                    );
                }
            }
        }

        if let Some(dmv) = &this.default_missing_value {
            if !this.optional_value {
                errors.err(
                    dmv,
                    "`default_missing_value` requires `optional_value` and may only be \
                     specified on `#[argy(option)]` fields",
                );
            }
        }

        if this.optional_value && this.default_missing_value.is_none() {
            errors.err(
                field,
                "`optional_value` requires a `default_missing_value` to use when the option \
                 is provided without a value",
            );
        }

        if !this.aliases.is_empty() {
            match this.field_type.as_ref().map(|f| f.kind) {
                Some(FieldKind::Option | FieldKind::Switch) => {
                    for alias in &this.aliases {
                        check_long_name(errors, alias, &alias.value());
                    }
                }
                _ => {
                    for alias in &this.aliases {
                        errors.err(
                            alias,
                            "`alias` may only be specified on `#[argy(option)]` \
                             or `#[argy(switch)]` fields",
                        );
                    }
                }
            }
        }

        match (&this.greedy, this.field_type.as_ref().map(|f| f.kind)) {
            (Some(_), Some(FieldKind::Positional)) => {}
            (Some(greedy), Some(_)) => errors.err(
                &greedy,
                "`greedy` may only be specified on `#[argy(positional)]` \
                    fields",
            ),
            _ => {}
        }

        if this.required {
            let is_greedy_positional = matches!(
                (this.field_type.as_ref().map(|f| f.kind), this.greedy.is_some()),
                (Some(FieldKind::Positional), true)
            );
            if !is_greedy_positional {
                if let Some(span) = required_span {
                    errors.err_span(
                        span,
                        "`required` may only be specified on `#[argy(positional, greedy)]` \
                            fields",
                    );
                }
            }
        }

        if this.global {
            match this.field_type.as_ref().map(|f| f.kind) {
                Some(FieldKind::Option | FieldKind::Switch) => {}
                _ => {
                    if let Some(span) = global_span {
                        errors.err_span(span, "`global` may only be specified on `#[argy(option)]` or `#[argy(switch)]` fields");
                    }
                }
            }
        }

        if !this.conflicts_with.is_empty() {
            match this.field_type.as_ref().map(|f| f.kind) {
                Some(FieldKind::Option | FieldKind::Switch) => {}
                _ => {
                    for conflict in &this.conflicts_with {
                        errors.err(
                            conflict,
                            "`conflicts_with` may only be specified on `#[argy(option)]` \
                             or `#[argy(switch)]` fields",
                        );
                    }
                }
            }
        }

        if let Some(env) = &this.env {
            match this.field_type.as_ref().map(|f| f.kind) {
                Some(FieldKind::Option | FieldKind::Switch) => {}
                _ => errors.err(
                    env,
                    "`env` may only be specified on `#[argy(option)]` \
                     or `#[argy(switch)]` fields",
                ),
            }
        }

        if let Some(d) = &this.description {
            check_option_description(errors, d.content.value().trim(), d.content.span());
        }

        this
    }

    fn parse_attr_from_str_fn(&mut self, errors: &Errors, m: &syn::MetaList) {
        parse_attr_fn_name(errors, m, "from_str_fn", &mut self.from_str_fn);
    }

    fn parse_attr_default(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        parse_attr_single_string(errors, m, "default", &mut self.default);
    }

    fn parse_attr_env(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        parse_attr_single_string(errors, m, "env", &mut self.env);
    }

    fn parse_attr_arg_name(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        parse_attr_single_string(errors, m, "arg_name", &mut self.arg_name);
    }

    fn parse_attr_long(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        parse_attr_single_string(errors, m, "long", &mut self.long);
        let long = self.long.as_ref().unwrap();
        let value = long.value();
        check_long_name(errors, long, &value);
    }

    fn parse_attr_short(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        if let Some(first) = &self.short {
            errors.duplicate_attrs("short", first, m);
        } else if let Some(lit_char) = errors.expect_lit_char(&m.value) {
            self.short = Some(lit_char.clone());
            if !lit_char.value().is_ascii() {
                errors.err(lit_char, "Short names must be ASCII");
            }
        }
    }
}

pub fn check_long_name(errors: &Errors, spanned: &impl syn::spanned::Spanned, value: &str) {
    if !value.is_ascii() {
        errors.err(spanned, "Long names must be ASCII");
    }
    if !value.chars().all(|c| c.is_lowercase() || c == '-' || c.is_ascii_digit()) {
        errors.err(spanned, "Long names may only contain lowercase letters, digits, and dashes");
    }
}

fn parse_attr_fn_name(
    errors: &Errors,
    m: &syn::MetaList,
    attr_name: &str,
    slot: &mut Option<syn::ExprPath>,
) {
    if let Some(first) = slot {
        errors.duplicate_attrs(attr_name, first, m);
    }

    *slot = errors.ok(m.parse_args());
}

fn parse_attr_field_type(
    errors: &Errors,
    meta: &syn::Meta,
    kind: FieldKind,
    slot: &mut Option<FieldType>,
) {
    if let Some(path) = errors.expect_meta_word(meta) {
        if let Some(first) = slot {
            errors.duplicate_attrs("field kind", &first.ident, path);
        } else if let Some(word) = path.get_ident() {
            *slot = Some(FieldType { kind, ident: word.clone() });
        }
    }
}

// Whether the attribute is one like `#[<name> ...]`
fn is_matching_attr(name: &str, attr: &syn::Attribute) -> bool {
    attr.path().segments.len() == 1 && attr.path().segments[0].ident == name
}

/// Checks for `#[doc ...]`, which is generated by doc comments.
fn is_doc_attr(attr: &syn::Attribute) -> bool {
    is_matching_attr("doc", attr)
}

/// Checks for `#[argy ...]`
fn is_argy_attr(attr: &syn::Attribute) -> bool {
    is_matching_attr("argy", attr)
}

/// Filters out non-`#[argy(...)]` attributes and converts to a sequence of `syn::Meta`.
fn argy_attr_to_meta_list(
    errors: &Errors,
    attr: &syn::Attribute,
) -> Option<impl IntoIterator<Item = syn::Meta>> {
    if !is_argy_attr(attr) {
        return None;
    }
    let ml = errors.expect_meta_list(&attr.meta)?;
    errors.ok(ml.parse_args_with(
        syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
    ))
}

/// Returns `true` if there are any `#[argy(...)]` attributes in the list.
pub fn has_argy_attrs(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(is_argy_attr)
}

/// Represents a `#[derive(FromArgs)]` type's top-level attributes.
#[derive(Default)]
pub struct TypeAttrs {
    pub is_subcommand: Option<syn::Ident>,
    pub repository: Option<syn::Ident>,
    pub homepage: Option<syn::Ident>,
    pub author: Option<syn::Ident>,
    pub name: Option<syn::LitStr>,
    pub short: Option<syn::LitChar>,
    pub description: Option<Description>,
    pub examples: Vec<syn::LitStr>,
    pub notes: Vec<syn::LitStr>,
    pub error_codes: Vec<(syn::LitInt, syn::LitStr)>,
    /// Arguments that trigger printing of the help message
    pub help_triggers: Option<Vec<syn::LitStr>>,
    /// Arguments that trigger printing of the crate name and version
    pub version_triggers: Option<Vec<syn::LitStr>>,
    pub usage: Option<syn::LitStr>,
    /// Alternative names for a subcommand.
    pub aliases: Vec<syn::LitStr>,
    /// Whether this subcommand should be hidden from help and completion output.
    pub hidden: bool,
}

impl TypeAttrs {
    /// Parse top-level `#[argy(...)]` attributes
    pub fn parse(errors: &Errors, derive_input: &syn::DeriveInput) -> Self {
        let mut this = Self::default();

        for attr in &derive_input.attrs {
            if is_doc_attr(attr) {
                parse_attr_doc(errors, attr, &mut this.description);
                continue;
            }

            let Some(ml) = argy_attr_to_meta_list(errors, attr) else {
                continue;
            };

            for meta in ml {
                let name = meta.path();
                if name.is_ident("alias") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_multi_string(errors, m, &mut this.aliases);
                    }
                } else if name.is_ident("description") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_description(errors, m, &mut this.description);
                    }
                } else if name.is_ident("error_code") {
                    if let Some(m) = errors.expect_meta_list(&meta) {
                        this.parse_attr_error_code(errors, m);
                    }
                } else if name.is_ident("example") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_example(errors, m);
                    }
                } else if name.is_ident("name") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_name(errors, m);
                    }
                } else if name.is_ident("short") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_short(errors, m);
                    }
                } else if name.is_ident("note") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_note(errors, m);
                    }
                } else if name.is_ident("repository") {
                    if let Some(ident) = errors.expect_meta_word(&meta).and_then(|p| p.get_ident())
                    {
                        this.parse_attr_repository(errors, ident);
                    }
                } else if name.is_ident("homepage") {
                    if let Some(ident) = errors.expect_meta_word(&meta).and_then(|p| p.get_ident())
                    {
                        this.parse_attr_homepage(errors, ident);
                    }
                } else if name.is_ident("author") {
                    if let Some(ident) = errors.expect_meta_word(&meta).and_then(|p| p.get_ident())
                    {
                        this.parse_attr_author(errors, ident);
                    }
                } else if name.is_ident("subcommand") {
                    if let Some(ident) = errors.expect_meta_word(&meta).and_then(|p| p.get_ident())
                    {
                        this.parse_attr_subcommand(errors, ident);
                    }
                } else if name.is_ident("help_triggers") {
                    if let Some(m) = errors.expect_meta_list(&meta) {
                        Self::parse_help_triggers(m, errors, &mut this);
                    }
                } else if name.is_ident("version_triggers") {
                    if let Some(m) = errors.expect_meta_list(&meta) {
                        Self::parse_version_triggers(m, errors, &mut this);
                    }
                } else if name.is_ident("usage") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        this.parse_attr_usage(errors, m);
                    }
                } else if name.is_ident("hidden") {
                    if errors.expect_meta_word(&meta).is_some() {
                        this.hidden = true;
                    }
                } else {
                    errors.err(
                        &meta,
                        concat!(
                            "Invalid type-level `argy` attribute\n",
                            "Expected one of: `alias`, `author`, `description`, `error_code`, `example`, `hidden`, `homepage`, ",
                            "`name`, `note`, `repository`, `short`, `subcommand`, `usage`, ",
                            "`help_triggers`, `version_triggers`",
                        ),
                    );
                }
            }
        }

        this.check_error_codes(errors);
        this
    }

    /// Checks that error codes are within range for `i32` and that they are
    /// never duplicated.
    fn check_error_codes(&self, errors: &Errors) {
        // map from error code to index
        let mut map: HashMap<u64, usize> = HashMap::new();
        for (index, (lit_int, _lit_str)) in self.error_codes.iter().enumerate() {
            let value = match lit_int.base10_parse::<u64>() {
                Ok(v) => v,
                Err(e) => {
                    errors.push(e);
                    continue;
                }
            };
            if value > (i32::MAX as u64) {
                errors.err(lit_int, "Error code out of range for `i32`");
            }
            match map.entry(value) {
                Entry::Occupied(previous) => {
                    let previous_index = *previous.get();
                    let (previous_lit_int, _previous_lit_str) = &self.error_codes[previous_index];
                    errors.err(lit_int, &format!("Duplicate error code {value}"));
                    errors.err(
                        previous_lit_int,
                        &format!("Error code {value} previously defined here"),
                    );
                }
                Entry::Vacant(slot) => {
                    slot.insert(index);
                }
            }
        }
    }

    fn parse_attr_error_code(&mut self, errors: &Errors, ml: &syn::MetaList) {
        errors.ok(ml.parse_args_with(|input: syn::parse::ParseStream| {
            let err_code = input.parse()?;
            input.parse::<syn::Token![,]>()?;
            let err_msg = input.parse()?;
            if let (Some(err_code), Some(err_msg)) =
                (errors.expect_lit_int(&err_code), errors.expect_lit_str(&err_msg))
            {
                self.error_codes.push((err_code.clone(), err_msg.clone()));
            }
            Ok(())
        }));
    }

    fn parse_attr_example(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        parse_attr_multi_string(errors, m, &mut self.examples);
    }

    fn parse_attr_name(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        parse_attr_single_string(errors, m, "name", &mut self.name);
        if let Some(name) = &self.name {
            if name.value() == "help" {
                errors.err(name, "Custom `help` commands are not supported.");
            }
        }
    }

    fn parse_attr_short(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        if let Some(first) = &self.short {
            errors.duplicate_attrs("short", first, m);
        } else if let Some(lit_char) = errors.expect_lit_char(&m.value) {
            self.short = Some(lit_char.clone());
            if !lit_char.value().is_ascii() {
                errors.err(lit_char, "Short names must be ASCII");
            }
        }
    }

    fn parse_attr_note(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        parse_attr_multi_string(errors, m, &mut self.notes);
    }

    fn parse_attr_subcommand(&mut self, errors: &Errors, ident: &syn::Ident) {
        if let Some(first) = &self.is_subcommand {
            errors.duplicate_attrs("subcommand", first, ident);
        } else {
            self.is_subcommand = Some(ident.clone());
        }
    }

    fn parse_attr_repository(&mut self, errors: &Errors, ident: &syn::Ident) {
        if let Some(first) = &self.repository {
            errors.duplicate_attrs("repository", first, ident);
        } else {
            self.repository = Some(ident.clone());
        }
    }

    fn parse_attr_homepage(&mut self, errors: &Errors, ident: &syn::Ident) {
        if let Some(first) = &self.homepage {
            errors.duplicate_attrs("homepage", first, ident);
        } else {
            self.homepage = Some(ident.clone());
        }
    }

    fn parse_attr_author(&mut self, errors: &Errors, ident: &syn::Ident) {
        if let Some(first) = &self.author {
            errors.duplicate_attrs("author", first, ident);
        } else {
            self.author = Some(ident.clone());
        }
    }

    // get the list of arguments that trigger printing of the help message as a vector of strings (help_arguments("-h", "--help", "help"))
    fn parse_help_triggers(m: &syn::MetaList, errors: &Errors, this: &mut Self) {
        let parser = Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        match parser.parse(m.tokens.clone().into()) {
            Ok(args) => {
                let mut triggers = Vec::new();
                for arg in args {
                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = arg {
                        triggers.push(lit_str);
                    }
                }

                this.help_triggers = Some(triggers);
            }
            Err(err) => errors.push(err),
        }
    }

    // get the list of arguments that trigger printing of the crate name and version as a vector of strings
    fn parse_version_triggers(m: &syn::MetaList, errors: &Errors, this: &mut Self) {
        let parser = Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated;
        match parser.parse(m.tokens.clone().into()) {
            Ok(args) => {
                let mut triggers = Vec::new();
                for arg in args {
                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(lit_str), .. }) = arg {
                        triggers.push(lit_str);
                    }
                }

                this.version_triggers = Some(triggers);
            }
            Err(err) => errors.push(err),
        }
    }

    fn parse_attr_usage(&mut self, errors: &Errors, m: &syn::MetaNameValue) {
        parse_attr_single_string(errors, m, "usage", &mut self.usage);
    }
}

/// Represents a `FromArgs` enum variant's attributes.
#[derive(Default)]
pub struct VariantAttrs {
    pub is_dynamic: Option<syn::Path>,
}

impl VariantAttrs {
    /// Parse enum variant `#[argy(...)]` attributes
    pub fn parse(errors: &Errors, variant: &syn::Variant) -> Self {
        let mut this = Self::default();

        let fields = match &variant.fields {
            syn::Fields::Named(fields) => Some(&fields.named),
            syn::Fields::Unnamed(fields) => Some(&fields.unnamed),
            syn::Fields::Unit => None,
        };

        for field in fields.into_iter().flatten() {
            for attr in &field.attrs {
                if is_argy_attr(attr) {
                    err_unused_enum_attr(errors, attr);
                }
            }
        }

        for attr in &variant.attrs {
            let Some(ml) = argy_attr_to_meta_list(errors, attr) else {
                continue;
            };

            for meta in ml {
                let name = meta.path();
                if name.is_ident("dynamic") {
                    if let Some(prev) = this.is_dynamic.as_ref() {
                        errors.duplicate_attrs("dynamic", prev, &meta);
                    } else {
                        this.is_dynamic = errors.expect_meta_word(&meta).cloned();
                    }
                } else {
                    errors.err(
                        &meta,
                        "Invalid variant-level `argy` attribute\n\
                         Subcommand variants can only have the #[argy(dynamic)] attribute.",
                    );
                }
            }
        }

        this
    }
}

/// Represents the attributes of a variant in a choice enum (an enum with `#[derive(FromArgValue)]`).
#[derive(Default)]
pub struct ChoiceVariantAttrs {
    pub name_override: Option<syn::LitStr>,
    /// Alternative string values that map to this variant.
    pub aliases: Vec<syn::LitStr>,
}

impl ChoiceVariantAttrs {
    /// Parse choice enum variant `#[argy(...)]` attributes
    pub fn parse(errors: &Errors, variant: &syn::Variant) -> Self {
        let mut this = Self::default();

        for attr in &variant.attrs {
            let Some(ml) = argy_attr_to_meta_list(errors, attr) else {
                continue;
            };

            for meta in ml {
                let name = meta.path();
                if name.is_ident("alias") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_multi_string(errors, m, &mut this.aliases);
                    }
                } else if name.is_ident("name") {
                    if let Some(m) = errors.expect_meta_name_value(&meta) {
                        parse_attr_single_string(errors, m, "name", &mut this.name_override);
                    }
                } else {
                    errors.err(
                        &meta,
                        "Invalid variant-level `argy` attribute\n\
                         Choice variants can only have the `name` or `alias` attribute.",
                    );
                }
            }
        }

        this
    }
}

/// The case used to render a `#[derive(ValueEnum)]` variant ident as its
/// string form.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum ValueCase {
    /// `FooBar` becomes `foo-bar` (the default, for clap parity).
    #[default]
    Kebab,
    /// `FooBar` becomes `foo_bar`.
    Snake,
}
impl ValueCase {
    pub const fn separator(self) -> &'static str {
        match self {
            Self::Kebab => "-",
            Self::Snake => "_",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "kebab_case" => Some(Self::Kebab),
            "snake_case" => Some(Self::Snake),
            _ => None,
        }
    }
}

/// Parse the optional `#[argy(rename_all = "snake_case"|"kebab_case")]`
/// attribute on a `#[derive(ValueEnum)]` enum. Unknown values produce an error
/// and fall back to the default (`kebab_case`).
pub fn parse_value_enum_rename_all(errors: &Errors, derive_input: &syn::DeriveInput) -> ValueCase {
    let mut value_case = ValueCase::default();
    for attr in &derive_input.attrs {
        let Some(ml) = argy_attr_to_meta_list(errors, attr) else {
            continue;
        };
        for meta in ml {
            if !meta.path().is_ident("rename_all") {
                continue;
            }
            let Some(m) = errors.expect_meta_name_value(&meta) else {
                continue;
            };
            let Some(lit) = errors.expect_lit_str(&m.value) else {
                continue;
            };
            match ValueCase::from_str(&lit.value()) {
                Some(c) => value_case = c,
                None => {
                    errors
                        .err(&m.value, "`rename_all` must be one of `snake_case` or `kebab_case`");
                }
            }
        }
    }
    value_case
}

fn check_option_description(errors: &Errors, desc: &str, span: Span) {
    let chars = &mut desc.trim().chars();
    match (chars.next(), chars.next()) {
        (Some(x), _) if x.is_lowercase() => {}
        // If both the first and second letter are not lowercase,
        // this is likely an initialism which should be allowed.
        (Some(x), Some(y)) if !x.is_lowercase() && (y.is_alphanumeric() && !y.is_lowercase()) => {}
        _ => {
            errors.err_span(span, "Descriptions must begin with a lowercase letter");
        }
    }
}

#[test]
fn test_initialisms() {
    use proc_macro2::TokenStream;
    use quote::ToTokens;
    use std::panic::Location;

    #[track_caller]
    fn check(s: &str, should_succeed: bool) {
        let errors = Errors::default();
        check_option_description(&errors, s, Span::call_site());

        let description_accepted = {
            let mut stream = TokenStream::new();
            errors.to_tokens(&mut stream);
            stream.is_empty()
        };

        assert!(
            description_accepted == should_succeed,
            "Assertion at {} failed",
            Location::caller(),
        );
    }

    check("Descriptions can't begin with an uppercase letter", false);
    check("descriptions must begin with a lowercase letter unless it's an initialism", true);
    check("HTTP is OK", true);
    check("I2C is OK", true);
    check("A sentence starting with a single-letter uppercase letter is bad even though it looks like an initialism", false);
    check("a sentence starting with a lowercase letter is good", true);
    check("非ラテン文字は常に受け入れられるべきです", true);

    // NOTE: Not so clear what should be done with this one, but I don't think anyone will ever
    // want to use I as the first word of a description anyway
    check(
        "I don't think 'I' should be accepted even though it's always grammatically expected to be
uppercase, like an initialism",
        false,
    );
}

#[test]
fn test_check_enum_type_attrs_span_by_value() {
    use proc_macro2::TokenStream;
    use quote::ToTokens;

    #[track_caller]
    fn produces_error(type_attrs: &TypeAttrs) -> bool {
        let errors = Errors::default();
        // `Span` is `Copy`; the function takes it by value. Passing a fresh
        // `call_site()` span directly would not compile if the signature
        // regressed back to `&Span`.
        check_enum_type_attrs(&errors, type_attrs, Span::call_site());

        let mut stream = TokenStream::new();
        errors.to_tokens(&mut stream);
        !stream.is_empty()
    }

    // An enum without `#[argy(subcommand)]` must report an error.
    assert!(produces_error(&TypeAttrs::default()));

    // An enum declaring `is_subcommand` must not report an error.
    let ok = TypeAttrs {
        is_subcommand: Some(syn::Ident::new("subcommand", Span::call_site())),
        ..Default::default()
    };
    assert!(!produces_error(&ok));
}

fn parse_attr_single_string(
    errors: &Errors,
    m: &syn::MetaNameValue,
    name: &str,
    slot: &mut Option<syn::LitStr>,
) {
    if let Some(first) = slot {
        errors.duplicate_attrs(name, first, m);
    } else if let Some(lit_str) = errors.expect_lit_str(&m.value) {
        *slot = Some(lit_str.clone());
    }
}

fn parse_attr_multi_string(errors: &Errors, m: &syn::MetaNameValue, list: &mut Vec<syn::LitStr>) {
    if let Some(lit_str) = errors.expect_lit_str(&m.value) {
        list.push(lit_str.clone());
    }
}

fn parse_attr_doc(errors: &Errors, attr: &syn::Attribute, slot: &mut Option<Description>) {
    let Some(nv) = errors.expect_meta_name_value(&attr.meta) else {
        return;
    };

    // Don't replace an existing explicit description.
    if slot.as_ref().is_some_and(|d| d.explicit) {
        return;
    }

    if let Some(lit_str) = errors.expect_lit_str(&nv.value) {
        let lit_str = if let Some(previous) = slot {
            let previous = &previous.content;
            let previous_span = previous.span();
            syn::LitStr::new(&(previous.value() + &unescape_doc(&lit_str.value())), previous_span)
        } else {
            syn::LitStr::new(&unescape_doc(&lit_str.value()), lit_str.span())
        };
        *slot = Some(Description { explicit: false, content: lit_str });
    }
}

/// Replaces escape sequences in doc-comments with the characters they represent.
///
/// Rustdoc understands `CommonMark` escape sequences consisting of a backslash followed by an ASCII
/// punctuation character. Any other backslash is treated as a literal backslash.
fn unescape_doc(s: &str) -> String {
    let mut result = String::with_capacity(s.len());

    let mut characters = s.chars().peekable();
    while let Some(mut character) = characters.next() {
        if character == '\\' {
            if let Some(next_character) = characters.peek() {
                if next_character.is_ascii_punctuation() {
                    character = *next_character;
                    characters.next();
                }
            }
        }

        // Braces must be escaped as this string will be used as a format string
        if character == '{' || character == '}' {
            result.push(character);
        }

        result.push(character);
    }

    result
}

fn parse_attr_description(errors: &Errors, m: &syn::MetaNameValue, slot: &mut Option<Description>) {
    let Some(lit_str) = errors.expect_lit_str(&m.value) else {
        return;
    };

    // Don't allow multiple explicit (non doc-comment) descriptions
    if let Some(description) = slot {
        if description.explicit {
            errors.duplicate_attrs("description", &description.content, lit_str);
        }
    }

    *slot = Some(Description { explicit: true, content: lit_str.clone() });
}

/// Checks that a `#![derive(FromArgs)]` enum has an `#[argy(subcommand)]`
/// attribute and that it does not have any other type-level `#[argy(...)]` attributes.
pub fn check_enum_type_attrs(errors: &Errors, type_attrs: &TypeAttrs, type_span: Span) {
    let TypeAttrs {
        is_subcommand,
        repository,
        homepage,
        author,
        name,
        short,
        description,
        examples,
        notes,
        error_codes,
        help_triggers,
        version_triggers,
        usage,
        aliases,
        hidden: _,
    } = type_attrs;

    // Ensure that `#[argy(subcommand)]` is present.
    if is_subcommand.is_none() {
        errors.err_span(
            type_span,
            concat!(
                "`#![derive(FromArgs)]` on `enum`s can only be used to enumerate subcommands.\n",
                "To enumerate subcommands, add `#[argy(subcommand)]` to the `enum` declaration.\n",
                "To declare a choice `enum` instead, use `#![derive(FromArgValue)]`."
            ),
        );
    }

    // Error on all other type-level attributes.
    if let Some(repository) = repository {
        err_unused_enum_attr(errors, repository);
    }
    if let Some(homepage) = homepage {
        err_unused_enum_attr(errors, homepage);
    }
    if let Some(author) = author {
        err_unused_enum_attr(errors, author);
    }
    if let Some(name) = name {
        err_unused_enum_attr(errors, name);
    }
    if let Some(short) = short {
        err_unused_enum_attr(errors, short);
    }
    if let Some(description) = description {
        if description.explicit {
            err_unused_enum_attr(errors, &description.content);
        }
    }
    if let Some(example) = examples.first() {
        err_unused_enum_attr(errors, example);
    }
    if let Some(note) = notes.first() {
        err_unused_enum_attr(errors, note);
    }
    if let Some(err_code) = error_codes.first() {
        err_unused_enum_attr(errors, &err_code.0);
    }
    if let Some(triggers) = help_triggers {
        if let Some(trigger) = triggers.first() {
            err_unused_enum_attr(errors, trigger);
        }
    }
    if let Some(triggers) = version_triggers {
        if let Some(trigger) = triggers.first() {
            err_unused_enum_attr(errors, trigger);
        }
    }
    if let Some(usage) = usage {
        err_unused_enum_attr(errors, usage);
    }
    if let Some(alias) = aliases.first() {
        err_unused_enum_attr(errors, alias);
    }
}

fn err_unused_enum_attr(errors: &Errors, location: &impl syn::spanned::Spanned) {
    errors.err(
        location,
        concat!(
            "Unused `argy` attribute on `#![derive(FromArgs)]` enum. ",
            "Such `enum`s can only be used to dispatch to subcommands, ",
            "and should only contain the #[argy(subcommand)] attribute.",
        ),
    );
}
