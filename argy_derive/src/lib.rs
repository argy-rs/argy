#![recursion_limit = "256"]
// Copyright (c) 2020 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use parse_attrs::has_argy_attrs;
use std::fmt::Write as _;
use syn::ext::IdentExt as _;

/// Implementation of the `FromArgs` and `argy(...)` derive attributes.
///
/// For more thorough documentation, see the `argy` crate itself.
extern crate proc_macro;

use {
    crate::{
        errors::Errors,
        parse_attrs::{check_long_name, FieldAttrs, FieldKind, TypeAttrs},
    },
    proc_macro2::{Span, TokenStream},
    quote::{quote, quote_spanned, ToTokens},
    std::{collections::HashMap, str::FromStr},
    syn::{spanned::Spanned, GenericArgument, LitStr, PathArguments, Type},
};

mod args_info;
mod errors;
mod help;
mod parse_attrs;

/// Entrypoint for `#[derive(FromArgs)]`.
#[proc_macro_derive(FromArgs, attributes(argy))]
pub fn argy_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let gen = impl_from_args(&ast);
    gen.into()
}

/// Entrypoint for `#[derive(FromArgValue)]`.
#[proc_macro_derive(FromArgValue, attributes(argy))]
pub fn argy_value_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let gen = impl_from_arg_value(&ast);
    gen.into()
}

/// Entrypoint for `#[derive(ArgsInfo)]`.
#[proc_macro_derive(ArgsInfo, attributes(argy))]
pub fn args_info_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let gen = args_info::impl_args_info(&ast);
    gen.into()
}

/// Entrypoint for `#[derive(ValueEnum)]`.
#[proc_macro_derive(ValueEnum, attributes(argy))]
pub fn value_enum_derive(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let ast = syn::parse_macro_input!(input as syn::DeriveInput);
    let gen = impl_value_enum(&ast);
    gen.into()
}

/// Transform the input into a token stream containing any generated implementations,
/// as well as all errors that occurred.
fn impl_from_args(input: &syn::DeriveInput) -> TokenStream {
    let errors = &Errors::default();
    let type_attrs = &TypeAttrs::parse(errors, input);
    let mut output_tokens = match &input.data {
        syn::Data::Struct(ds) => {
            impl_from_args_struct(errors, &input.ident, type_attrs, &input.generics, ds)
        }
        syn::Data::Enum(de) => {
            impl_from_args_enum(errors, &input.ident, type_attrs, &input.generics, de)
        }
        syn::Data::Union(_) => {
            errors.err(input, "`#[derive(FromArgs)]` cannot be applied to unions");
            TokenStream::new()
        }
    };
    errors.to_tokens(&mut output_tokens);
    output_tokens
}

fn impl_from_arg_value(input: &syn::DeriveInput) -> TokenStream {
    let errors = &Errors::default();
    let mut output_tokens = if let syn::Data::Enum(de) = &input.data {
        impl_from_arg_value_enum(errors, &input.ident, &input.generics, de)
    } else {
        errors.err(input, "`#[derive(FromArgValue)]` can only be applied to `enum`s");
        TokenStream::new()
    };
    if has_argy_attrs(&input.attrs) {
        errors.err(
            &input.ident,
            "`#[derive(FromArgValue)]` `enum`s do not support `#[argy(...)]` attributes",
        );
    }
    errors.to_tokens(&mut output_tokens);
    output_tokens
}

/// Transform the input into a token stream containing any generated
/// implementations for a `#[derive(ValueEnum)]` enum, as well as all errors
/// that occurred.
fn impl_value_enum(input: &syn::DeriveInput) -> TokenStream {
    let errors = &Errors::default();
    let mut output_tokens = if let syn::Data::Enum(de) = &input.data {
        impl_value_enum_enum(errors, &input.ident, &input.generics, input, de)
    } else {
        errors.err(input, "`#[derive(ValueEnum)]` can only be applied to `enum`s");
        TokenStream::new()
    };
    errors.to_tokens(&mut output_tokens);
    output_tokens
}

/// Implements `std::str::FromStr`, `std::fmt::Display`, `argy::ValueEnum`, and
/// (via the blanket `FromStr` impl) `argy::FromArgValue` for a fieldless
/// `#![derive(ValueEnum)]` enum.
// Too many lines: this helper builds a large generated token stream.
#[allow(clippy::too_many_lines)]
fn impl_value_enum_enum(
    errors: &Errors,
    name: &syn::Ident,
    generic_args: &syn::Generics,
    input: &syn::DeriveInput,
    de: &syn::DataEnum,
) -> TokenStream {
    // A fieldless enum variant with its canonical string name and aliases.
    struct ValueVariant<'a> {
        ident: &'a syn::Ident,
        name: syn::LitStr,
        aliases: Vec<syn::LitStr>,
    }

    let value_case = parse_attrs::parse_value_enum_rename_all(errors, input);
    let sep = value_case.separator();

    let variants: Vec<ValueVariant<'_>> = de
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            choice_enum_only_fieldless_variant(errors, &variant.fields);
            let attrs = parse_attrs::ChoiceVariantAttrs::parse(errors, variant);
            let name = attrs.name_override.unwrap_or_else(|| {
                let name_str = pascal_to_case(&format!("{ident}"), sep);
                syn::LitStr::new(&name_str, ident.span())
            });
            ValueVariant { ident, name, aliases: attrs.aliases }
        })
        .collect();

    if variants.is_empty() {
        errors.err(&de.variants, "Value enums must have at least one variant");
    }

    let name_repeating = std::iter::repeat(name.clone());
    let variant_idents = variants.iter().map(|x| x.ident).collect::<Vec<_>>();
    let variant_names = variants.iter().map(|x| &x.name).collect::<Vec<_>>();
    // A `|`-separated pattern per variant that accepts the canonical name and
    // any aliases.
    let variant_match_patterns = variants.iter().map(|variant| {
        let variant_name = &variant.name;
        let mut pattern = quote! { #variant_name };
        for alias in &variant.aliases {
            pattern = quote! { #pattern | #alias };
        }
        pattern
    });
    let err_literal = {
        let mut err = "expected ".to_string();
        for (i, vname) in variant_names.iter().enumerate() {
            if i == 0 {
            } else if i == variant_names.len() - 1 {
                err.push_str(" or ");
            } else {
                err.push_str(", ");
            }
            let _ = write!(err, "{:?}", vname.value());
        }
        LitStr::new(&err, name.span())
    };
    // The `&[Self]` value list, in declaration order.
    let variant_slice = quote! { &[ #( #name::#variant_idents ),* ] };
    // A `to_possible_value` arm per variant.
    let possible_values = variants.iter().map(|variant| {
        let vident = variant.ident;
        let vname = &variant.name;
        let aliases = &variant.aliases;
        let alias_arr = if aliases.is_empty() {
            quote! { &[] }
        } else {
            quote! { &[ #( #aliases ),* ] }
        };
        quote! {
            #name::#vident => argy::ValueEnumValue::new(#vname, #alias_arr)
        }
    });
    // A `Display` arm per variant.
    let display_arms = variants.iter().map(|variant| {
        let vident = variant.ident;
        let vname = &variant.name;
        quote! {
            #name::#vident => ::core::write!(f, "{}", #vname)
        }
    });
    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();
    quote! {
        impl #impl_generics ::core::str::FromStr for #name #ty_generics #where_clause {
            type Err = String;
            fn from_str(value: &str) -> ::core::result::Result<Self, Self::Err> {
                ::core::result::Result::Ok(match value {
                    #(
                        #variant_match_patterns => #name_repeating::#variant_idents,
                    )*
                    _ => {
                        return ::core::result::Result::Err(#err_literal.to_owned())
                    }
                })
            }
        }
        impl #impl_generics ::core::fmt::Display for #name #ty_generics #where_clause {
            fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                match self {
                    #(
                        #display_arms,
                    )*
                }
            }
        }
        impl #impl_generics argy::ValueEnum for #name #ty_generics #where_clause {
            fn value_variants() -> &'static [Self] {
                #variant_slice
            }
            fn to_possible_value(&self) -> ::core::option::Option<argy::ValueEnumValue> {
                ::core::option::Option::Some(match self {
                    #(
                        #possible_values,
                    )*
                })
            }
        }
    }
}
/// The kind of optionality a parameter has.
enum Optionality {
    None,
    Defaulted(TokenStream),
    Optional,
    Repeating,
    DefaultedRepeating(TokenStream),
}

impl PartialEq<Self> for Optionality {
    fn eq(&self, other: &Self) -> bool {
        use Optionality::{Optional, Repeating};
        // NB: (Defaulted, Defaulted) can't contain the same token streams
        matches!((self, other), (Optional, Optional) | (Repeating, Repeating))
    }
}

impl Optionality {
    /// Whether or not this is `Optionality::None`
    const fn is_required(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// A field of a `#![derive(FromArgs)]` struct with attributes and some other
/// notable metadata appended.
struct StructField<'a> {
    /// The original parsed field
    field: &'a syn::Field,
    /// The parsed attributes of the field
    attrs: FieldAttrs,
    /// The field name. This is contained optionally inside `field`,
    /// but is duplicated non-optionally here to indicate that all field that
    /// have reached this point must have a field name, and it no longer
    /// needs to be unwrapped.
    name: &'a syn::Ident,
    /// Similar to `name` above, this is contained optionally inside `FieldAttrs`,
    /// but here is fully present to indicate that we only have to consider fields
    /// with a valid `kind` at this point.
    kind: FieldKind,
    // If `field.ty` is `Vec<T>` or `Option<T>`, this is `T`, otherwise it's `&field.ty`.
    // This is used to enable consistent parsing code between optional and non-optional
    // keyed and subcommand fields.
    ty_without_wrapper: &'a syn::Type,
    // Whether the field represents an optional value, such as an `Option` subcommand field
    // or an `Option` or `Vec` keyed argument, or if it has a `default`.
    optionality: Optionality,
    // The `--`-prefixed name of the option, if one exists.
    long_name: Option<String>,
}

impl<'a> StructField<'a> {
    /// Attempts to parse a field of a `#[derive(FromArgs)]` struct, pulling out the
    /// fields required for code generation.
    #[allow(clippy::too_many_lines)]
    fn new(errors: &Errors, field: &'a syn::Field, attrs: FieldAttrs) -> Option<Self> {
        let name = field.ident.as_ref().expect("missing ident for named field");
        // A flattened field inlines a nested `FromArgs` struct; it has no kind.
        if attrs.flatten {
            return Some(StructField {
                field,
                attrs,
                name,
                kind: FieldKind::Flatten,
                ty_without_wrapper: &field.ty,
                optionality: Optionality::None,
                long_name: None,
            });
        }

        // Ensure that one "kind" is present (switch, option, subcommand, positional)
        let kind = if let Some(field_type) = &attrs.field_type {
            field_type.kind
        } else {
            errors.err(
                field,
                concat!(
                    "Missing `argy` field kind attribute.\n",
                    "Expected one of: `switch`, `option`, `remaining`, `subcommand`, `positional`",
                ),
            );
            return None;
        };

        // Parse out whether a field is optional (`Option` or `Vec`).
        let optionality;
        let ty_without_wrapper;
        match kind {
            FieldKind::Switch => {
                if !ty_expect_switch(errors, &field.ty) {
                    return None;
                }
                optionality = Optionality::Optional;
                ty_without_wrapper = &field.ty;
            }
            FieldKind::Option | FieldKind::Positional => {
                if let Some(default) = &attrs.default {
                    let Ok(tokens) = TokenStream::from_str(&default.value()) else {
                        errors.err(&default, "Invalid tokens: unable to lex `default` value");
                        return None;
                    };
                    // Set the span of the generated tokens to the string literal
                    let tokens: TokenStream = tokens
                        .into_iter()
                        .map(|mut tree| {
                            tree.set_span(default.span());
                            tree
                        })
                        .collect();
                    let inner = if let Some(x) = ty_inner(&["Vec"], &field.ty) {
                        optionality = Optionality::DefaultedRepeating(tokens);
                        x
                    } else {
                        optionality = Optionality::Defaulted(tokens);
                        &field.ty
                    };
                    ty_without_wrapper = inner;
                } else {
                    let mut inner = None;
                    optionality = if let Some(x) = ty_inner(&["Option"], &field.ty) {
                        inner = Some(x);
                        Optionality::Optional
                    } else if let Some(x) = ty_inner(&["Vec"], &field.ty) {
                        inner = Some(x);
                        Optionality::Repeating
                    } else {
                        Optionality::None
                    };
                    ty_without_wrapper = inner.unwrap_or(&field.ty);
                }
            }
            FieldKind::SubCommand => {
                let inner = ty_inner(&["Option"], &field.ty);
                optionality =
                    if inner.is_some() { Optionality::Optional } else { Optionality::None };
                ty_without_wrapper = inner.unwrap_or(&field.ty);
            }
            FieldKind::Flatten => unreachable!(),
        }

        // Determine the "long" name of options and switches.
        // Defaults to the kebab-case'd field name if `#[argy(long = "...")]` is omitted.
        let long_name = match kind {
            FieldKind::Switch | FieldKind::Option => {
                let long_name = attrs.long.as_ref().map_or_else(
                    || {
                        let kebab_name = to_kebab_case(&name.unraw().to_string());
                        check_long_name(errors, name, &kebab_name);
                        kebab_name
                    },
                    syn::LitStr::value,
                );
                if long_name == "help" {
                    errors.err(field, "Custom `--help` flags are not supported.");
                }
                let long_name = format!("--{long_name}");
                Some(long_name)
            }
            FieldKind::SubCommand | FieldKind::Positional | FieldKind::Flatten => None,
        };

        if let Some(env) = &attrs.env {
            match kind {
                FieldKind::Option
                    if matches!(
                        optionality,
                        Optionality::Repeating | Optionality::DefaultedRepeating(_)
                    ) =>
                {
                    errors.err(
                        env,
                        "`env` may not be specified on repeating `#[argy(option)]` fields",
                    );
                }
                _ => {}
            }
        }

        if attrs.optional_value {
            match kind {
                FieldKind::Option
                    if matches!(
                        optionality,
                        Optionality::Repeating | Optionality::DefaultedRepeating(_)
                    ) =>
                {
                    errors.err(
                        field,
                        "`optional_value` may not be specified on repeating \
                         `#[argy(option)]` fields",
                    );
                }
                _ => {}
            }
        }

        if let Some(vd) = &attrs.value_delimiter {
            match kind {
                FieldKind::Option
                    if matches!(
                        optionality,
                        Optionality::Repeating | Optionality::DefaultedRepeating(_)
                    ) => {}
                _ => {
                    errors.err(
                        vd,
                        "`value_delimiter` may only be specified on repeating \
                         (Vec) `#[argy(option)]` fields",
                    );
                }
            }
        }

        Some(StructField { field, attrs, name, kind, ty_without_wrapper, optionality, long_name })
    }

    pub(crate) fn positional_arg_name(&self) -> String {
        self.attrs
            .arg_name
            .as_ref()
            .map_or_else(|| self.name.to_string().trim_matches('_').to_owned(), LitStr::value)
    }
}

fn to_kebab_case(s: &str) -> String {
    let words = s.split('_').filter(|word| !word.is_empty());
    let mut res = String::with_capacity(s.len());
    for word in words {
        if !res.is_empty() {
            res.push('-');
        }
        res.push_str(word);
    }
    res
}

#[test]
fn test_kebabs() {
    #[track_caller]
    fn check(s: &str, want: &str) {
        let got = to_kebab_case(s);
        assert_eq!(got.as_str(), want);
    }
    check("", "");
    check("_", "");
    check("foo", "foo");
    check("__foo_", "foo");
    check("foo_bar", "foo-bar");
    check("foo__Bar", "foo-Bar");
    check("foo_bar__baz_", "foo-bar-baz");
}

/// Implements `FromArgs` and `TopLevelCommand` or `SubCommand` for a `#[derive(FromArgs)]` struct.
fn impl_from_args_struct(
    errors: &Errors,
    name: &syn::Ident,
    type_attrs: &TypeAttrs,
    generic_args: &syn::Generics,
    ds: &syn::DataStruct,
) -> TokenStream {
    let fields = match &ds.fields {
        syn::Fields::Named(fields) => fields,
        syn::Fields::Unnamed(_) => {
            errors.err(
                &ds.struct_token,
                "`#![derive(FromArgs)]` is not currently supported on tuple structs",
            );
            return TokenStream::new();
        }
        syn::Fields::Unit => {
            errors.err(&ds.struct_token, "#![derive(FromArgs)]` cannot be applied to unit structs");
            return TokenStream::new();
        }
    };

    // Split out `#[argy(flatten)]` fields: their nested `FromArgs` fields are
    // inlined via a runtime parse contribution rather than as direct slots, so
    // the static table builders below operate only on the regular fields.
    let (flatten_fields, fields): (Vec<_>, Vec<_>) = fields
        .named
        .iter()
        .filter_map(|field| {
            let attrs = FieldAttrs::parse(errors, field);
            StructField::new(errors, field, attrs)
        })
        .partition(|f| f.kind == FieldKind::Flatten);

    ensure_unique_names(errors, &fields);
    ensure_only_last_positional_is_optional(errors, &fields);
    ensure_last_positional_valid(errors, &fields);

    let impl_span = Span::call_site();

    let from_args_method =
        impl_from_args_struct_from_args(errors, type_attrs, &fields, &flatten_fields);

    let redact_arg_values_method =
        impl_from_args_struct_redact_arg_values(errors, type_attrs, &fields, &flatten_fields);

    let top_or_sub_cmd_impl = top_or_sub_cmd_impl(errors, name, type_attrs, generic_args);

    let flatten_contribution_impl =
        impl_flatten_contribution(errors, name, generic_args, &fields, &flatten_fields);

    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();
    let trait_impl = quote_spanned! { impl_span =>
        #[automatically_derived]
        impl #impl_generics argy::FromArgs for #name #ty_generics #where_clause {
            #from_args_method

            #redact_arg_values_method
        }

        #top_or_sub_cmd_impl

        #flatten_contribution_impl
    };

    trait_impl
}

// Too many lines: this helper builds a large generated token stream.
#[allow(clippy::too_many_lines)]
fn impl_from_args_struct_from_args<'a>(
    errors: &Errors,
    type_attrs: &TypeAttrs,
    fields: &'a [StructField<'a>],
    flatten_fields: &'a [StructField<'a>],
) -> TokenStream {
    if !flatten_fields.is_empty() {
        return impl_from_args_struct_from_args_flatten(errors, type_attrs, fields, flatten_fields);
    }
    let init_fields = declare_local_storage_for_from_args_fields(fields);
    let unwrap_fields = unwrap_from_args_fields(fields);
    let positional_fields: Vec<&StructField<'_>> =
        fields.iter().filter(|field| field.kind == FieldKind::Positional).collect();
    let positional_field_idents = positional_fields.iter().map(|field| &field.field.ident);
    let positional_field_names = positional_fields.iter().map(|field| field.name.to_string());
    let last_positional_is_repeating =
        positional_fields.last().is_some_and(|field| field.optionality == Optionality::Repeating);
    let last_positional_is_greedy = positional_fields
        .last()
        .is_some_and(|field| field.kind == FieldKind::Positional && field.attrs.greedy.is_some());

    let flag_output_table = fields.iter().filter_map(|field| {
        let field_name = &field.field.ident;
        match field.kind {
            FieldKind::Option if field.attrs.optional_value => {
                let missing = field
                    .attrs
                    .default_missing_value
                    .as_ref()
                    .expect("optional_value requires default_missing_value");
                let missing_value = missing.value();
                Some(quote! {
                    argy::ParseStructOption::OptionalValue {
                        slot: &mut #field_name,
                        missing_value: #missing_value,
                    }
                })
            }
            FieldKind::Option => Some(quote! { argy::ParseStructOption::Value(&mut #field_name) }),
            FieldKind::Switch => Some(quote! { argy::ParseStructOption::Flag(&mut #field_name) }),
            FieldKind::SubCommand | FieldKind::Positional | FieldKind::Flatten => None,
        }
    });

    let flag_str_to_output_table_map = flag_str_to_output_table_map_entries(fields);
    let global_options = global_options_entries(fields);
    let conflicts = conflicts_entries_tokens(fields, errors);
    let num_option_slots = fields
        .iter()
        .filter(|field| matches!(field.kind, FieldKind::Option | FieldKind::Switch))
        .count();

    let mut subcommands_iter =
        fields.iter().filter(|field| field.kind == FieldKind::SubCommand).fuse();

    let subcommand: Option<&StructField<'_>> = subcommands_iter.next();
    for dup_subcommand in subcommands_iter {
        errors.duplicate_attrs("subcommand", subcommand.unwrap().field, dup_subcommand.field);
    }

    let impl_span = Span::call_site();

    let missing_requirements_ident = syn::Ident::new("__missing_requirements", impl_span);

    let env_fill = env_fill_fields(fields);

    let append_missing_requirements =
        append_missing_requirements(&missing_requirements_ident, fields);

    let requires_check = requires_check_tokens(fields, errors, &missing_requirements_ident);

    let parse_subcommands = subcommand.map_or_else(
        || quote_spanned! { impl_span => None },
        |subcommand| {
            let name = subcommand.name;
            let ty = subcommand.ty_without_wrapper;
            quote_spanned! { impl_span =>
                Some(argy::ParseStructSubCommand {
                    subcommands: <#ty as argy::SubCommands>::COMMANDS,
                    dynamic_subcommands: &<#ty as argy::SubCommands>::dynamic_commands(),
                    parse_func: Box::new(|__command, __remaining_args| {
                        #name = Some(<#ty as argy::FromArgs>::from_args(__command, __remaining_args)?);
                        ::core::result::Result::Ok(())
                    }),
                })
            }
        },
    );

    let help_triggers = get_help_triggers(type_attrs);
    let parse_help_triggers = get_parse_help_triggers(type_attrs);
    let version_triggers = get_version_triggers(type_attrs);
    let version_func = version_func(type_attrs);

    let help = if cfg!(feature = "help") {
        // Identifier referring to a value containing the name of the current command as an `&[&str]`.
        let cmd_name_str_array_ident = syn::Ident::new("__cmd_name", impl_span);
        help::help(
            errors,
            cmd_name_str_array_ident,
            type_attrs,
            fields,
            &[],
            subcommand,
            &help_triggers,
        )
    } else {
        quote! { String::new() }
    };

    let method_impl = quote_spanned! { impl_span =>
        fn from_args(__cmd_name: &[&str], __args: &[&str])
            -> ::core::result::Result<Self, argy::EarlyExit>
        {
            #![allow(clippy::unwrap_in_result)]

            #( #init_fields )*

            let mut __seen = [false; #num_option_slots];

            let __usage = #help;

            argy::parse_struct_args(
                __cmd_name,
                __args,
                argy::ParseStructOptions {
                    arg_to_slot: &[ #( #flag_str_to_output_table_map ,)* ],
                    slots: &mut [ #( #flag_output_table, )* ],
                    seen: &mut __seen,
                    conflicts: &[ #( #conflicts ,)* ],
                    help_triggers: &[ #( #parse_help_triggers ),* ],
                    version_triggers: &[ #( #version_triggers ),* ],
                    global_options: &[ #( #global_options ),* ],
                },
                argy::ParseStructPositionals {
                    positionals: &mut [
                        #(
                            argy::ParseStructPositional {
                                name: #positional_field_names,
                                slot: &mut #positional_field_idents as &mut argy::ParseValueSlot,
                            },
                        )*
                    ],
                    last_is_repeating: #last_positional_is_repeating,
                    last_is_greedy: #last_positional_is_greedy,
                },
                #parse_subcommands,
                &|| __usage.clone(),
                #version_func,
            )?;

            #( #env_fill )*

            let mut #missing_requirements_ident = argy::MissingRequirements::default();
            #(
                #append_missing_requirements
            )*
            #( #requires_check )*
            #missing_requirements_ident.err_on_any(&__usage)?;

            ::core::result::Result::Ok(Self {

                #( #unwrap_fields, )*
            })
        }
    };

    method_impl
}

/// get help triggers vector from `type_attrs.help_triggers` as a [`Vec<String>`]
///
/// Defaults to vec!["--help", "help"] if `type_attrs.help_triggers` is None
fn get_help_triggers(type_attrs: &TypeAttrs) -> Vec<String> {
    let help_triggers = type_attrs.help_triggers.as_ref().map_or_else(
        || vec!["--help".to_owned(), "help".to_owned()],
        |s| {
            s.iter()
                .filter_map(|s| {
                    let trigger = s.value();
                    let trigger_trimmed = trigger.trim().to_owned();
                    if trigger_trimmed.is_empty() {
                        None
                    } else {
                        Some(trigger_trimmed)
                    }
                })
                .collect::<Vec<_>>()
        },
    );
    help_triggers
}

/// get the help triggers used for parsing: the display help triggers plus `-h`
/// when using the default set, so `-h` is accepted as a short form of help like
/// in clap without changing the rendered help text. Explicitly provided
/// `help_triggers` are used verbatim for parsing.
fn get_parse_help_triggers(type_attrs: &TypeAttrs) -> Vec<String> {
    let mut help_triggers = get_help_triggers(type_attrs);
    if type_attrs.help_triggers.is_none() && !help_triggers.iter().any(|t| t == "-h") {
        help_triggers.push("-h".to_owned());
    }
    help_triggers
}
///
/// Get version triggers vector from `type_attrs.version_triggers` as a [`Vec<String>`].
///
/// Defaults to vec!["--version", "-V"] if `type_attrs.version_triggers` is None, so
/// `-V` is accepted as a short form of `--version` like in clap.
fn get_version_triggers(type_attrs: &TypeAttrs) -> Vec<String> {
    let version_triggers = type_attrs.version_triggers.as_ref().map_or_else(
        || vec!["--version".to_owned(), "-V".to_owned()],
        |s| {
            s.iter()
                .filter_map(|s| {
                    let trigger = s.value();
                    let trigger_trimmed = trigger.trim().to_owned();
                    if trigger_trimmed.is_empty() {
                        None
                    } else {
                        Some(trigger_trimmed)
                    }
                })
                .collect::<Vec<_>>()
        },
    );
    version_triggers
}

/// Generate the `version_func` closure passed to [`argy::parse_struct_args`].
///
/// For a subcommand this prints a subcommand-qualified name like clap's
/// `zoxide-add 0.10.0` (i.e. `<crate>-<subcommand> <version>`), while the
/// top-level command keeps printing `<crate> <version>`. The subcommand name is
/// the last element of the runtime `cmd_name`, which the subcommand parser
/// appends when dispatching.
fn version_func(type_attrs: &TypeAttrs) -> TokenStream {
    if type_attrs.is_subcommand.is_some() {
        quote! {
            &|__cmd_name: &[&str]| {
                let __name = match __cmd_name.last() {
                    Some(__subcommand) => {
                        let mut __name = ::core::env!("CARGO_PKG_NAME").to_owned();
                        __name.push('-');
                        __name.push_str(__subcommand);
                        __name
                    }
                    None => ::core::env!("CARGO_PKG_NAME").to_owned(),
                };
                ::core::format_args!("{} {}", __name, ::core::env!("CARGO_PKG_VERSION")).to_string()
            }
        }
    } else {
        quote! {
            &|_| ::core::format_args!("{} {}", ::core::env!("CARGO_PKG_NAME"), ::core::env!("CARGO_PKG_VERSION")).to_string()
        }
    }
}

// Too many lines: this helper builds a large generated token stream.
#[allow(clippy::too_many_lines)]
fn impl_from_args_struct_from_args_flatten<'a>(
    errors: &Errors,
    type_attrs: &TypeAttrs,
    fields: &'a [StructField<'a>],
    flatten_fields: &'a [StructField<'a>],
) -> TokenStream {
    let init_fields = declare_local_storage_for_from_args_fields(fields);
    let unwrap_fields = unwrap_from_args_fields(fields);
    let positional_fields: Vec<&StructField<'_>> =
        fields.iter().filter(|field| field.kind == FieldKind::Positional).collect();
    let positional_field_idents = positional_fields.iter().map(|field| &field.field.ident);
    let positional_field_names = positional_fields.iter().map(|field| field.name.to_string());
    let last_positional_is_repeating =
        positional_fields.last().is_some_and(|field| field.optionality == Optionality::Repeating);
    let last_positional_is_greedy = positional_fields
        .last()
        .is_some_and(|field| field.kind == FieldKind::Positional && field.attrs.greedy.is_some());

    let flag_output_table = fields.iter().filter_map(|field| {
        let field_name = &field.field.ident;
        match field.kind {
            FieldKind::Option if field.attrs.optional_value => {
                let missing = field
                    .attrs
                    .default_missing_value
                    .as_ref()
                    .expect("optional_value requires default_missing_value");
                let missing_value = missing.value();
                Some(quote! {
                    argy::ParseStructOption::OptionalValue {
                        slot: &mut #field_name,
                        missing_value: #missing_value,
                    }
                })
            }
            FieldKind::Option => Some(quote! { argy::ParseStructOption::Value(&mut #field_name) }),
            FieldKind::Switch => Some(quote! { argy::ParseStructOption::Flag(&mut #field_name) }),
            FieldKind::SubCommand | FieldKind::Positional | FieldKind::Flatten => None,
        }
    });

    let flag_str_to_output_table_map = flag_str_to_output_table_map_entries(fields);
    let global_options = global_options_entries(fields);
    let conflicts = conflicts_entries_tokens(fields, errors);
    let num_option_slots = fields
        .iter()
        .filter(|field| matches!(field.kind, FieldKind::Option | FieldKind::Switch))
        .count();

    let mut subcommands_iter =
        fields.iter().filter(|field| field.kind == FieldKind::SubCommand).fuse();
    let subcommand: Option<&StructField<'_>> = subcommands_iter.next();
    for dup_subcommand in subcommands_iter {
        errors.duplicate_attrs("subcommand", subcommand.unwrap().field, dup_subcommand.field);
    }

    let impl_span = Span::call_site();

    let missing_requirements_ident = syn::Ident::new("__missing_requirements", impl_span);

    let env_fill = env_fill_fields(fields);
    let append_missing_requirements =
        append_missing_requirements(&missing_requirements_ident, fields);
    let requires_check = requires_check_tokens(fields, errors, &missing_requirements_ident);

    let parse_subcommands = subcommand.map_or_else(
        || quote_spanned! { impl_span => None },
        |subcommand| {
            let name = subcommand.name;
            let ty = subcommand.ty_without_wrapper;
            quote_spanned! { impl_span =>
                Some(argy::ParseStructSubCommand {
                    subcommands: <#ty as argy::SubCommands>::COMMANDS,
                    dynamic_subcommands: &<#ty as argy::SubCommands>::dynamic_commands(),
                    parse_func: Box::new(|__command, __remaining_args| {
                        #name = Some(<#ty as argy::FromArgs>::from_args(__command, __remaining_args)?);
                        ::core::result::Result::Ok(())
                    }),
                })
            }
        },
    );

    let help_triggers = get_help_triggers(type_attrs);
    let parse_help_triggers = get_parse_help_triggers(type_attrs);
    let version_triggers = get_version_triggers(type_attrs);
    let version_func = version_func(type_attrs);

    let help = if cfg!(feature = "help") {
        // Identifier referring to a value containing the name of the current command as an `&[&str]`.
        let cmd_name_str_array_ident = syn::Ident::new("__cmd_name", impl_span);
        help::help(
            errors,
            cmd_name_str_array_ident,
            type_attrs,
            fields,
            flatten_fields,
            subcommand,
            &help_triggers,
        )
    } else {
        quote! { String::new() }
    };

    // Per-flatten-field append/build/struct-field code, plus the shared table
    // declarations and the post-parse drop that releases the contributions.
    let mut flatten_decl = Vec::new();
    let mut flatten_append = Vec::new();
    let mut flatten_build = Vec::new();
    let mut flatten_struct_fields = Vec::new();
    let mut flatten_check_missing = Vec::new();
    for (i, fl) in flatten_fields.iter().enumerate() {
        let ty = fl.ty_without_wrapper;
        let field_ident = fl.name;
        let decl_ident = syn::Ident::new(&format!("__flatten_{i}"), impl_span);
        let seen_base_ident = syn::Ident::new(&format!("__seen_base_{i}"), impl_span);
        let n_ident = syn::Ident::new(&format!("__flatten_n_{i}"), impl_span);
        let val_ident = syn::Ident::new(&format!("__flatten_val_{i}"), impl_span);
        flatten_decl.push(quote_spanned! { impl_span =>
            let mut #decl_ident = <#ty as argy::FlattenFromArgs>::flatten_contribution();
        });
        flatten_append.push(quote_spanned! { impl_span =>
            let #seen_base_ident = __seen.len();
            let #n_ident = argy::FlattenContribution::append(
                &mut #decl_ident,
                &mut __arg_to_slot,
                &mut __slots,
                #seen_base_ident,
                &mut __positionals,
                &mut __last_is_repeating,
                &mut __last_is_greedy,
                &mut __subcommand,
            );
            __seen.resize(__seen.len() + #n_ident, false);
        });
        flatten_build.push(quote_spanned! { impl_span =>
            let #val_ident = <_ as argy::FlattenContribution<#ty>>::build(#decl_ident);
        });
        flatten_struct_fields.push(quote_spanned! { impl_span =>
            #field_ident: #val_ident,
        });
        flatten_check_missing.push(quote_spanned! { impl_span =>
            <_ as argy::FlattenContribution<#ty>>::check_missing(
                &#decl_ident,
                &mut #missing_requirements_ident,
            );
        });
    }

    let method_impl = quote_spanned! { impl_span =>
        fn from_args(__cmd_name: &[&str], __args: &[&str])
            -> ::core::result::Result<Self, argy::EarlyExit>
        {
            #![allow(clippy::unwrap_in_result)]

            #( #init_fields )*

            let mut __arg_to_slot: ::std::vec::Vec<(&'static str, usize)> = ::std::vec![
                #( #flag_str_to_output_table_map ,)*
            ];
            let mut __slots: ::std::vec::Vec<argy::ParseStructOption> = ::std::vec![
                #( #flag_output_table ,)*
            ];
            let mut __positionals: ::std::vec::Vec<argy::ParseStructPositional> = ::std::vec![
                #(
                    argy::ParseStructPositional {
                        name: #positional_field_names,
                        slot: &mut #positional_field_idents as &mut argy::ParseValueSlot,
                    },
                )*
            ];
            let mut __last_is_repeating = #last_positional_is_repeating;
            let mut __last_is_greedy = #last_positional_is_greedy;
            let mut __seen: ::std::vec::Vec<bool> = ::std::vec![false; #num_option_slots];

            #( #flatten_decl )*

            let mut __subcommand: ::std::option::Option<argy::ParseStructSubCommand> =
                #parse_subcommands;

            #( #flatten_append )*

            let __usage = #help;

            argy::parse_struct_args(
                __cmd_name,
                __args,
                argy::ParseStructOptions {
                    arg_to_slot: &__arg_to_slot[..],
                    slots: &mut __slots[..],
                    seen: &mut __seen[..],
                    conflicts: &[ #( #conflicts ,)* ],
                    help_triggers: &[ #( #parse_help_triggers ),* ],
                    version_triggers: &[ #( #version_triggers ),* ],
                    global_options: &[ #( #global_options ),* ],
                },
                argy::ParseStructPositionals {
                    positionals: &mut __positionals[..],
                    last_is_repeating: __last_is_repeating,
                    last_is_greedy: __last_is_greedy,
                },
                __subcommand,
                &|| __usage.clone(),
                #version_func,
            )?;

            #( #env_fill )*

            // Release the borrows the flatten contributions hold on the shared
            // tables before building the nested values.
            ::core::mem::drop((__arg_to_slot, __slots, __positionals));

            let mut #missing_requirements_ident = argy::MissingRequirements::default();
            #(
                #append_missing_requirements
            )*
            #( #requires_check )*
            #( #flatten_check_missing )*
            #missing_requirements_ident.err_on_any(&__usage)?;

            #( #flatten_build )*

            ::core::result::Result::Ok(Self {
                #( #unwrap_fields, )*
                #( #flatten_struct_fields )*
            })
        }
    };

    method_impl
}

// Generates `FlattenFromArgs` + `FlattenContribution` impls so this type can be
// inlined into a parent command via `#[argy(flatten)]`. The contribution owns the
// same slots the type's own `from_args` would, and `append` merges them into a
// parent's shared tables.
// Too many lines: this helper builds a large generated token stream.
#[allow(clippy::too_many_lines)]
fn impl_flatten_contribution<'a>(
    errors: &Errors,
    name: &syn::Ident,
    generic_args: &syn::Generics,
    fields: &'a [StructField<'a>],
    flatten_fields: &'a [StructField<'a>],
) -> TokenStream {
    let contribution_ident =
        syn::Ident::new(&format!("{name}FlattenContribution"), Span::call_site());
    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();

    // ---- Struct fields of the contribution ----
    let struct_fields = fields.iter().map(|field| {
        let fname = field.name;
        match field.kind {
            FieldKind::Option | FieldKind::Positional => {
                let field_type = field.ty_without_wrapper;
                let field_slot_type = match field.optionality {
                    Optionality::Optional | Optionality::Repeating => {
                        (&field.field.ty).into_token_stream()
                    }
                    Optionality::None | Optionality::Defaulted(_) => {
                        quote! { std::option::Option<#field_type> }
                    }
                    Optionality::DefaultedRepeating(_) => {
                        quote! { std::option::Option<std::vec::Vec<#field_type>> }
                    }
                };
                quote! { #fname: argy::ParseValueSlotTy<#field_slot_type, #field_type> }
            }
            FieldKind::Switch => {
                let field_type = &field.field.ty;
                quote! { #fname: #field_type }
            }
            FieldKind::SubCommand => {
                // The contribution holds the subcommand as `Option` (starting
                // `None`) and unwraps it in `build` for required subcommands.
                let field_type = &field.ty_without_wrapper;
                quote! { #fname: std::option::Option<#field_type> }
            }
            FieldKind::Flatten => unreachable!(),
        }
    });
    let nested_struct_fields = flatten_fields.iter().map(|field| {
        let fname = field.name;
        let ty = field.ty_without_wrapper;
        quote! { #fname: <#ty as argy::FlattenFromArgs>::Contribution }
    });

    // ---- Constructor field initializers ----
    let ctor_fields = fields.iter().map(|field| {
        let fname = field.name;
        match field.kind {
            FieldKind::Option | FieldKind::Positional => {
                let field_type = field.ty_without_wrapper;
                let from_str_fn = field.attrs.from_str_fn.as_ref().map_or_else(
                    || {
                        quote! {
                            <#field_type as argy::FromArgValue>::from_arg_value
                        }
                    },
                    ToTokens::into_token_stream,
                );
                let value_delimiter = field.attrs.value_delimiter.as_ref().map_or_else(
                    || quote! { ::core::option::Option::None },
                    |c| quote! { ::core::option::Option::Some(#c) },
                );
                quote! {
                    #fname: argy::ParseValueSlotTy {
                        slot: std::default::Default::default(),
                        parse_func: |_, value| { #from_str_fn(value) },
                        value_delimiter: #value_delimiter,
                    }
                }
            }
            FieldKind::Switch => quote! { #fname: argy::Flag::default() },
            FieldKind::SubCommand => quote! { #fname: None },
            FieldKind::Flatten => unreachable!(),
        }
    });
    let nested_ctor_fields = flatten_fields.iter().map(|field| {
        let fname = field.name;
        let ty = field.ty_without_wrapper;
        quote! { #fname: <#ty as argy::FlattenFromArgs>::flatten_contribution() }
    });

    // ---- append(): option/switch entries ----
    let option_blocks: Vec<TokenStream> = fields
        .iter()
        .filter(|field| matches!(field.kind, FieldKind::Option | FieldKind::Switch))
        .map(|field| {
            let fname = field.name;
            let slot_entries = flag_str_to_output_table_map_entries_for_contribution(field);
            let parse_option = match field.kind {
                FieldKind::Option if field.attrs.optional_value => {
                    let missing = field
                        .attrs
                        .default_missing_value
                        .as_ref()
                        .expect("optional_value requires default_missing_value");
                    let missing_value = missing.value();
                    quote! {
                        argy::ParseStructOption::OptionalValue {
                            slot: &mut self.#fname,
                            missing_value: #missing_value,
                        }
                    }
                }
                FieldKind::Option => {
                    quote! { argy::ParseStructOption::Value(&mut self.#fname) }
                }
                FieldKind::Switch => {
                    quote! { argy::ParseStructOption::Flag(&mut self.#fname) }
                }
                _ => unreachable!(),
            };
            quote! {
                #slot_entries
                slots.push(#parse_option);
                __slot += 1;
            }
        })
        .collect();

    // ---- append(): positionals ----
    let positional_fields: Vec<&StructField<'_>> =
        fields.iter().filter(|f| f.kind == FieldKind::Positional).collect();
    let positional_block = if positional_fields.is_empty() {
        TokenStream::new()
    } else {
        let pos_entries = positional_fields.iter().map(|field| {
            let fname = field.name;
            let name = field.positional_arg_name();
            quote! {
                positionals.push(argy::ParseStructPositional {
                    name: #name,
                    slot: &mut self.#fname as &mut argy::ParseValueSlot,
                });
            }
        });
        let last_repeating =
            positional_fields.last().is_some_and(|f| f.optionality == Optionality::Repeating);
        let last_greedy = positional_fields.last().is_some_and(|f| f.attrs.greedy.is_some());
        quote! {
            #( #pos_entries )*
            *last_is_repeating = #last_repeating;
            *last_is_greedy = #last_greedy;
        }
    };

    // ---- append(): subcommand ----
    let subcommand_block = {
        let mut subcommands_iter = fields.iter().filter(|f| f.kind == FieldKind::SubCommand).fuse();
        let subcommand = subcommands_iter.next();
        subcommand.map_or_else(TokenStream::new, |subcommand| {
            let fname = subcommand.name;
            let ty = subcommand.ty_without_wrapper;
            quote! {
                if subcommand.is_none() {
                    let __sub_ref = &mut self.#fname;
                    *subcommand = Some(argy::ParseStructSubCommand {
                        subcommands: <#ty as argy::SubCommands>::COMMANDS,
                        dynamic_subcommands: &<#ty as argy::SubCommands>::dynamic_commands(),
                        parse_func: Box::new(move |__command, __remaining_args| {
                            *__sub_ref = Some(<#ty as argy::FromArgs>::from_args(__command, __remaining_args)?);
                            ::core::result::Result::Ok(())
                        }),
                    });
                }
            }
        })
    };

    // ---- append(): nested flatten ----
    let nested_blocks: Vec<TokenStream> = flatten_fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let fname = field.name;
            let n_ident = syn::Ident::new(&format!("__nested_n_{i}"), Span::call_site());
            quote! {
                let #n_ident = self.#fname.append(
                    arg_to_slot,
                    slots,
                    __slot,
                    positionals,
                    last_is_repeating,
                    last_is_greedy,
                    subcommand,
                );
                __slot += #n_ident;
            }
        })
        .collect();

    // ---- build(): field value construction ----
    let build_fields = fields.iter().map(|field| {
        let fname = field.name;
        match field.kind {
            FieldKind::Option | FieldKind::Positional => match &field.optionality {
                Optionality::None => quote! { #fname: self.#fname.slot.unwrap() },
                Optionality::Optional | Optionality::Repeating => {
                    quote! { #fname: self.#fname.slot }
                }
                Optionality::Defaulted(tokens) | Optionality::DefaultedRepeating(tokens) => {
                    quote! {
                        #fname: self.#fname.slot.unwrap_or_else(|| #tokens)
                    }
                }
            },
            FieldKind::Switch => quote! { #fname: self.#fname },
            FieldKind::SubCommand => match field.optionality {
                Optionality::None => quote! { #fname: self.#fname.unwrap() },
                Optionality::Optional | Optionality::Repeating => quote! { #fname: self.#fname },
                Optionality::Defaulted(_) | Optionality::DefaultedRepeating(_) => unreachable!(),
            },
            FieldKind::Flatten => unreachable!(),
        }
    });
    let nested_build_fields = flatten_fields.iter().map(|field| {
        let fname = field.name;
        let ty = field.ty_without_wrapper;
        quote! { #fname: <_ as argy::FlattenContribution<#ty>>::build(self.#fname) }
    });

    // ---- check_missing(): report required fields that were not provided ----
    let check_missing_own = fields.iter().filter(|f| f.optionality.is_required()).map(|field| {
        let fname = field.name;
        match field.kind {
            FieldKind::Option => {
                let name = field.long_name.as_ref().expect("options always have a long name");
                quote! {
                    if self.#fname.slot.is_none() {
                        mri.missing_option(#name);
                    }
                }
            }
            FieldKind::Positional => {
                let name = field.positional_arg_name();
                quote! {
                    if self.#fname.slot.is_none() {
                        mri.missing_positional_arg(#name);
                    }
                }
            }
            FieldKind::SubCommand => {
                let ty = field.ty_without_wrapper;
                quote! {
                    if self.#fname.is_none() {
                        mri.missing_subcommands(
                            <#ty as argy::SubCommands>::COMMANDS
                                .iter()
                                .cloned()
                                .chain(
                                    <#ty as argy::SubCommands>::dynamic_commands()
                                        .iter()
                                        .copied()
                                ),
                        );
                    }
                }
            }
            FieldKind::Switch | FieldKind::Flatten => unreachable!(),
        }
    });
    let check_missing_nested = flatten_fields.iter().map(|field| {
        let fname = field.name;
        quote! { self.#fname.check_missing(mri); }
    });

    // Static help lines for this type's own fields plus recursive calls to any

    // Static help lines for this type's own fields plus recursive calls to any
    // nested flattened types, inlined at the parent command's scope.
    let static_fragment = crate::help::flatten_help_fragment(errors, fields);
    let nested_fragments = flatten_fields.iter().map(|field| {
        let ty = field.ty_without_wrapper;
        quote! { __frag.push_str(&<#ty as argy::FlattenFromArgs>::flatten_help_fragment()); }
    });

    quote! {
        #[automatically_derived]
        #[doc(hidden)]
        struct #contribution_ident #impl_generics #where_clause {
            #( #struct_fields, )*
            #( #nested_struct_fields, )*
        }

        #[automatically_derived]
        impl #impl_generics argy::FlattenFromArgs for #name #ty_generics #where_clause {
            type Contribution = #contribution_ident #ty_generics;

            fn flatten_contribution() -> Self::Contribution {
                #contribution_ident {
                    #( #ctor_fields, )*
                    #( #nested_ctor_fields, )*
                }
            }

            fn flatten_help_fragment() -> String {
                let mut __frag = String::from(#static_fragment);
                #( #nested_fragments )*
                __frag
            }
        }

        #[automatically_derived]
        impl #impl_generics argy::FlattenContribution<#name #ty_generics> for #contribution_ident #ty_generics #where_clause {
            #[allow(clippy::too_many_arguments)]
            fn append<'a>(
                &'a mut self,
                arg_to_slot: &mut Vec<(&'static str, usize)>,
                slots: &mut Vec<argy::ParseStructOption<'a>>,
                seen_base: usize,
                positionals: &mut Vec<argy::ParseStructPositional<'a>>,
                last_is_repeating: &mut bool,
                last_is_greedy: &mut bool,
                subcommand: &mut Option<argy::ParseStructSubCommand<'a>>,
            ) -> usize {
                let mut __slot = seen_base;
                #( #option_blocks )*
                #positional_block
                #subcommand_block
                #( #nested_blocks )*
                __slot - seen_base
            }

            fn build(self) -> #name #ty_generics {
                #name {
                    #( #build_fields, )*
                    #( #nested_build_fields, )*
                }
            }

            fn check_missing(&self, mri: &mut argy::MissingRequirements) {
                #( #check_missing_own )*
                #( #check_missing_nested )*
            }
        }
    }
}

// Too many lines: this helper builds a large generated token stream.
#[allow(clippy::too_many_lines)]
fn impl_from_args_struct_redact_arg_values<'a>(
    errors: &Errors,
    type_attrs: &TypeAttrs,
    fields: &'a [StructField<'a>],
    _flatten_fields: &'a [StructField<'a>],
) -> TokenStream {
    let init_fields = declare_local_storage_for_redacted_fields(fields);
    let unwrap_fields = unwrap_redacted_fields(fields);

    let positional_fields: Vec<&StructField<'_>> =
        fields.iter().filter(|field| field.kind == FieldKind::Positional).collect();
    let positional_field_idents = positional_fields.iter().map(|field| &field.field.ident);
    let positional_field_names = positional_fields.iter().map(|field| field.name.to_string());
    let last_positional_is_repeating =
        positional_fields.last().is_some_and(|field| field.optionality == Optionality::Repeating);
    let last_positional_is_greedy = positional_fields
        .last()
        .is_some_and(|field| field.kind == FieldKind::Positional && field.attrs.greedy.is_some());

    let flag_output_table = fields.iter().filter_map(|field| {
        let field_name = &field.field.ident;
        match field.kind {
            FieldKind::Option if field.attrs.optional_value => {
                let missing = field
                    .attrs
                    .default_missing_value
                    .as_ref()
                    .expect("optional_value requires default_missing_value");
                let missing_value = missing.value();
                Some(quote! {
                    argy::ParseStructOption::OptionalValue {
                        slot: &mut #field_name,
                        missing_value: #missing_value,
                    }
                })
            }
            FieldKind::Option => Some(quote! { argy::ParseStructOption::Value(&mut #field_name) }),
            FieldKind::Switch => Some(quote! { argy::ParseStructOption::Flag(&mut #field_name) }),
            FieldKind::SubCommand | FieldKind::Positional | FieldKind::Flatten => None,
        }
    });

    let flag_str_to_output_table_map = flag_str_to_output_table_map_entries(fields);
    let global_options = global_options_entries(fields);
    let conflicts = conflicts_entries_tokens(fields, errors);
    let num_option_slots = fields
        .iter()
        .filter(|field| matches!(field.kind, FieldKind::Option | FieldKind::Switch))
        .count();

    let mut subcommands_iter =
        fields.iter().filter(|field| field.kind == FieldKind::SubCommand).fuse();

    let subcommand: Option<&StructField<'_>> = subcommands_iter.next();
    for dup_subcommand in subcommands_iter {
        errors.duplicate_attrs("subcommand", subcommand.unwrap().field, dup_subcommand.field);
    }

    let impl_span = Span::call_site();

    let missing_requirements_ident = syn::Ident::new("__missing_requirements", impl_span);

    let append_missing_requirements =
        append_missing_requirements(&missing_requirements_ident, fields);

    let redact_subcommands = subcommand.map_or_else(
        || quote_spanned! { impl_span => None },
        |subcommand| {
            let name = subcommand.name;
            let ty = subcommand.ty_without_wrapper;
            quote_spanned! { impl_span =>
                Some(argy::ParseStructSubCommand {
                    subcommands: <#ty as argy::SubCommands>::COMMANDS,
                    dynamic_subcommands: &<#ty as argy::SubCommands>::dynamic_commands(),
                    parse_func: Box::new(|__command, __remaining_args| {
                        #name = Some(<#ty as argy::FromArgs>::redact_arg_values(__command, __remaining_args)?);
                        ::core::result::Result::Ok(())
                    }),
                })
            }
        },
    );

    let unwrap_cmd_name_err_string = if type_attrs.is_subcommand.is_none() {
        quote! { "no command name" }
    } else {
        quote! { "no subcommand name" }
    };

    let help_triggers = get_help_triggers(type_attrs);
    let parse_help_triggers = get_parse_help_triggers(type_attrs);
    let version_triggers = get_version_triggers(type_attrs);
    let version_func = version_func(type_attrs);

    let help = if cfg!(feature = "help") {
        // Identifier referring to a value containing the name of the current command as an `&[&str]`.
        let cmd_name_str_array_ident = syn::Ident::new("__cmd_name", impl_span);
        help::help(
            errors,
            cmd_name_str_array_ident,
            type_attrs,
            fields,
            &[],
            subcommand,
            &help_triggers,
        )
    } else {
        quote! { String::new() }
    };

    let method_impl = quote_spanned! { impl_span =>
        fn redact_arg_values(__cmd_name: &[&str], __args: &[&str]) -> std::result::Result<Vec<String>, argy::EarlyExit> {
            #( #init_fields )*

            let mut __seen = [false; #num_option_slots];

            let __usage = #help;

            argy::parse_struct_args(
                __cmd_name,
                __args,
                argy::ParseStructOptions {
                    arg_to_slot: &[ #( #flag_str_to_output_table_map ,)* ],
                    slots: &mut [ #( #flag_output_table, )* ],
                    seen: &mut __seen,
                    conflicts: &[ #( #conflicts ,)* ],
                    help_triggers: &[ #( #parse_help_triggers ),* ],
                    version_triggers: &[ #( #version_triggers ),* ],
                    global_options: &[ #( #global_options ),* ],
                },
                argy::ParseStructPositionals {
                    positionals: &mut [
                        #(
                            argy::ParseStructPositional {
                                name: #positional_field_names,
                                slot: &mut #positional_field_idents as &mut argy::ParseValueSlot,
                            },
                        )*
                    ],
                    last_is_repeating: #last_positional_is_repeating,
                    last_is_greedy: #last_positional_is_greedy,
                },
                #redact_subcommands,
                &|| __usage.clone(),
                #version_func,
            )?;

            let mut #missing_requirements_ident = argy::MissingRequirements::default();
            #(
                #append_missing_requirements
            )*
            #missing_requirements_ident.err_on_any(&__usage)?;

            let mut __redacted = vec![
                if let Some(cmd_name) = __cmd_name.last() {
                    (*cmd_name).to_owned()
                } else {
                    return ::core::result::Result::Err(argy::EarlyExit::from(#unwrap_cmd_name_err_string.to_owned()));
                }
            ];

            #( #unwrap_fields )*

            ::core::result::Result::Ok(__redacted)
        }
    };

    method_impl
}

/// Ensures that only the last positional arg is non-required.
fn ensure_only_last_positional_is_optional(errors: &Errors, fields: &[StructField<'_>]) {
    let mut first_non_required_span = None;
    for field in fields {
        if field.kind == FieldKind::Positional {
            if let Some(first) = first_non_required_span {
                errors.err_span(
                    first,
                    "Only the last positional argument may be `Option`, `Vec`, or defaulted.",
                );
                errors.err(&field.field, "Later positional argument declared here.");
                return;
            }
            if !field.optionality.is_required() {
                first_non_required_span = Some(field.field.span());
            }
        }
    }
}

/// Ensures that a `#[argy(positional, last)]` field is the final positional
/// argument and is a repeating `Vec` (clap `last` parity).
fn ensure_last_positional_valid(errors: &Errors, fields: &[StructField<'_>]) {
    let positionals: Vec<&StructField<'_>> =
        fields.iter().filter(|f| f.kind == FieldKind::Positional).collect();
    for (i, field) in positionals.iter().enumerate() {
        if !field.attrs.last {
            continue;
        }
        if i != positionals.len() - 1 {
            errors
                .err(&field.field, "`last` may only be specified on the last positional argument.");
        }
        if !matches!(field.optionality, Optionality::Repeating | Optionality::DefaultedRepeating(_))
        {
            errors.err(
                &field.field,
                "`last` may only be specified on a repeating (`Vec`) positional argument.",
            );
        }
    }
}

/// Ensures that only one short or long name is used.
fn ensure_unique_names(errors: &Errors, fields: &[StructField<'_>]) {
    let mut seen_short_names = HashMap::new();
    let mut seen_long_names = HashMap::new();

    for field in fields {
        if let Some(short_name) = &field.attrs.short {
            let short_name = short_name.value();
            if let Some(first_use_field) = seen_short_names.get(&short_name) {
                errors.err_span_tokens(
                    first_use_field,
                    &format!("The short name of \"-{short_name}\" was already used here."),
                );
                errors.err_span_tokens(field.field, "Later usage here.");
            }

            seen_short_names.insert(short_name, &field.field);
        }

        if let Some(long_name) = &field.long_name {
            if let Some(first_use_field) = seen_long_names.get(&long_name) {
                errors.err_span_tokens(
                    *first_use_field,
                    &format!("The long name of \"{long_name}\" was already used here."),
                );
                errors.err_span_tokens(field.field, "Later usage here.");
            }

            seen_long_names.insert(long_name, field.field);
        }
    }
}

/// Implement `argy::TopLevelCommand` or `argy::SubCommand` as appropriate.
fn top_or_sub_cmd_impl(
    errors: &Errors,
    name: &syn::Ident,
    type_attrs: &TypeAttrs,
    generic_args: &syn::Generics,
) -> TokenStream {
    let description = if cfg!(feature = "help") {
        help::require_description(errors, name.span(), &type_attrs.description, "type")
    } else {
        String::new()
    };
    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();
    if type_attrs.is_subcommand.is_none() {
        // Not a subcommand
        quote! {
            #[automatically_derived]
            impl #impl_generics argy::TopLevelCommand for #name #ty_generics #where_clause {}
        }
    } else {
        let empty_str = syn::LitStr::new("", Span::call_site());
        let subcommand_name = type_attrs.name.as_ref().unwrap_or_else(|| {
            errors.err(name, "`#[argy(name = \"...\")]` attribute is required for subcommands");
            &empty_str
        });
        let short_name =
            type_attrs.short.as_ref().map_or_else(|| quote! { &'\0' }, |c| quote! { &#c });
        let aliases =
            type_attrs.aliases.iter().map(|lit| syn::LitStr::new(&lit.value(), lit.span()));
        let aliases = quote! { &[#( #aliases, )*] };
        let hidden = type_attrs.hidden;
        quote! {
            #[automatically_derived]
            impl #impl_generics argy::SubCommand for #name #ty_generics #where_clause {
                const COMMAND: &'static argy::CommandInfo = &argy::CommandInfo {
                    name: #subcommand_name,
                    short: #short_name,
                    description: #description,
                    aliases: #aliases,
                    hidden: #hidden,
                };
            }
        }
    }
}

/// Declare a local slots to store each field in during parsing.
///
/// Most fields are stored in `Option<FieldType>` locals.
/// `argy(option)` fields are stored in a `ParseValueSlotTy` along with a
/// function that knows how to decode the appropriate value.
fn declare_local_storage_for_from_args_fields<'a>(
    fields: &'a [StructField<'a>],
) -> impl Iterator<Item = TokenStream> + 'a {
    fields.iter().map(|field| {
        let field_name = &field.field.ident;
        let field_type = &field.ty_without_wrapper;

        // Wrap field types in `Option` if they aren't already `Option` or `Vec`-wrapped.
        let field_slot_type = match field.optionality {
            Optionality::Optional | Optionality::Repeating => (&field.field.ty).into_token_stream(),
            Optionality::None | Optionality::Defaulted(_) => {
                quote! { std::option::Option<#field_type> }
            }
            Optionality::DefaultedRepeating(_) => {
                quote! { std::option::Option<std::vec::Vec<#field_type>> }
            }
        };

        match field.kind {
            FieldKind::Option | FieldKind::Positional => {
                let from_str_fn = field.attrs.from_str_fn.as_ref().map_or_else(
                    || {
                        quote! {
                            <#field_type as argy::FromArgValue>::from_arg_value
                        }
                    },
                    ToTokens::into_token_stream,
                );
                let value_delimiter = field.attrs.value_delimiter.as_ref().map_or_else(
                    || quote! { ::core::option::Option::None },
                    |c| quote! { ::core::option::Option::Some(#c) },
                );
                quote! {
                    let mut #field_name: argy::ParseValueSlotTy<#field_slot_type, #field_type>
                        = argy::ParseValueSlotTy {
                            slot: std::default::Default::default(),
                            parse_func: |_, value| { #from_str_fn(value) },
                            value_delimiter: #value_delimiter,
                        };
                }
            }
            FieldKind::SubCommand => {
                quote! { let mut #field_name: #field_slot_type = None; }
            }
            FieldKind::Switch => {
                quote! { let mut #field_name: #field_slot_type = argy::Flag::default(); }
            }
            FieldKind::Flatten => unreachable!(),
        }
    })
}

/// Unwrap non-optional fields and take options out of their tuple slots.
fn unwrap_from_args_fields<'a>(
    fields: &'a [StructField<'a>],
) -> impl Iterator<Item = TokenStream> + 'a {
    fields.iter().map(|field| {
        let field_name = field.name;
        match field.kind {
            FieldKind::Option | FieldKind::Positional => match &field.optionality {
                Optionality::None => quote! {
                    #field_name: #field_name.slot.unwrap()
                },
                Optionality::Optional | Optionality::Repeating => {
                    quote! { #field_name: #field_name.slot }
                }
                Optionality::Defaulted(tokens) | Optionality::DefaultedRepeating(tokens) => {
                    quote! {
                        #field_name: #field_name.slot.unwrap_or_else(|| #tokens)
                    }
                }
            },
            FieldKind::Switch => field_name.into_token_stream(),
            FieldKind::SubCommand => match field.optionality {
                Optionality::None => quote! { #field_name: #field_name.unwrap() },
                Optionality::Optional | Optionality::Repeating => field_name.into_token_stream(),
                Optionality::Defaulted(_) | Optionality::DefaultedRepeating(_) => unreachable!(),
            },
            FieldKind::Flatten => unreachable!(),
        }
    })
}

/// Fill `env`-sourced option/switch slots that were not provided on the command
/// line. Runs after CLI parsing so a CLI value always takes precedence, and
/// before missing-requirement checks so a required option satisfied by its env
/// var is not reported missing. Each statement references the matching slot in
/// the `__seen` array (indexed by option/switch field order).
fn env_fill_fields<'a>(fields: &'a [StructField<'a>]) -> Vec<TokenStream> {
    let mut out = Vec::new();
    let mut option_slot_index = 0usize;
    for field in fields {
        match field.kind {
            FieldKind::Option | FieldKind::Switch => {
                let idx = option_slot_index;
                option_slot_index += 1;
                let Some(env) = &field.attrs.env else {
                    continue;
                };
                let name = field.name;
                let env_lit = syn::LitStr::new(&env.value(), env.span());
                let env_name = &env.value();
                match field.kind {
                    FieldKind::Option => {
                        // Non-repeating options are guaranteed by `StructField::new`.
                        out.push(quote! {
                            if !__seen[#idx] {
                                if let ::core::result::Result::Ok(__env_val) =
                                    ::std::env::var(#env_lit)
                                {
                                    if let ::core::result::Result::Ok(__env_parsed) =
                                        (#name.parse_func)(#env_name, &__env_val)
                                    {
                                        #name.slot = ::core::option::Option::Some(__env_parsed);
                                    }
                                }
                            }
                        });
                    }
                    FieldKind::Switch => {
                        out.push(quote! {
                            if !__seen[#idx] {
                                if let ::core::result::Result::Ok(__env_val) =
                                    ::std::env::var(#env_lit)
                                {
                                    let __env_truthy = match __env_val.to_ascii_lowercase().as_str()
                                    {
                                        "0" | "false" | "f" | "no" | "n" | "off" | "" => false,
                                        _ => true,
                                    };
                                    if __env_truthy {
                                        argy::Flag::set_flag(&mut #name);
                                    }
                                }
                            }
                        });
                    }
                    FieldKind::Flatten | FieldKind::SubCommand | FieldKind::Positional => {
                        unreachable!()
                    }
                }
            }
            FieldKind::SubCommand | FieldKind::Positional | FieldKind::Flatten => {}
        }
    }
    out
}

/// Declare a local slots to store each field in during parsing.
///
/// Most fields are stored in `Option<FieldType>` locals.
/// `argy(option)` fields are stored in a `ParseValueSlotTy` along with a
/// function that knows how to decode the appropriate value.
fn declare_local_storage_for_redacted_fields<'a>(
    fields: &'a [StructField<'a>],
) -> impl Iterator<Item = TokenStream> + 'a {
    fields.iter().map(|field| {
        let field_name = &field.field.ident;

        match field.kind {
            FieldKind::Switch => {
                quote! {
                    let mut #field_name = argy::RedactFlag {
                        slot: None,
                    };
                }
            }
            FieldKind::Option => {
                let field_slot_type = match field.optionality {
                    Optionality::Repeating => {
                        quote! { std::vec::Vec<String> }
                    }
                    Optionality::DefaultedRepeating(_) => {
                        quote! { std::option::Option<std::vec::Vec<String>> }
                    }
                    Optionality::None | Optionality::Optional | Optionality::Defaulted(_) => {
                        quote! { std::option::Option<String> }
                    }
                };

                quote! {
                    let mut #field_name: argy::ParseValueSlotTy::<#field_slot_type, String> =
                        argy::ParseValueSlotTy {
                        slot: std::default::Default::default(),
                        parse_func: |arg, _| { ::core::result::Result::Ok(arg.to_owned()) },
                        value_delimiter: ::core::option::Option::None,
                    };
                }
            }
            FieldKind::Positional => {
                let field_slot_type = match field.optionality {
                    Optionality::Repeating => {
                        quote! { std::vec::Vec<String> }
                    }
                    Optionality::DefaultedRepeating(_) => {
                        quote! { std::option::Option<std::vec::Vec<String>> }
                    }
                    Optionality::None | Optionality::Optional | Optionality::Defaulted(_) => {
                        quote! { std::option::Option<String> }
                    }
                };

                let arg_name = field.positional_arg_name();
                quote! {
                    let mut #field_name: argy::ParseValueSlotTy::<#field_slot_type, String> =
                        argy::ParseValueSlotTy {
                        slot: std::default::Default::default(),
                        parse_func: |_, _| { ::core::result::Result::Ok(#arg_name.to_owned()) },
                        value_delimiter: ::core::option::Option::None,
                    };
                }
            }
            FieldKind::SubCommand => {
                quote! { let mut #field_name: std::option::Option<std::vec::Vec<String>> = None; }
            }
            FieldKind::Flatten => unreachable!(),
        }
    })
}

/// Unwrap non-optional fields and take options out of their tuple slots.
fn unwrap_redacted_fields<'a>(
    fields: &'a [StructField<'a>],
) -> impl Iterator<Item = TokenStream> + 'a {
    fields.iter().map(|field| {
        let field_name = field.name;

        match field.kind {
            FieldKind::Switch => {
                quote! {
                    if let Some(__field_name) = #field_name.slot {
                        __redacted.push(__field_name);
                    }
                }
            }
            FieldKind::Option => match field.optionality {
                Optionality::Repeating => {
                    quote! {
                        __redacted.extend(#field_name.slot.into_iter());
                    }
                }
                Optionality::DefaultedRepeating(_) => {
                    quote! {
                        if let Some(__field_name) = #field_name.slot {
                            __redacted.extend(__field_name.into_iter());
                        }
                    }
                }
                Optionality::None | Optionality::Optional | Optionality::Defaulted(_) => {
                    quote! {
                        if let Some(__field_name) = #field_name.slot {
                            __redacted.push(__field_name);
                        }
                    }
                }
            },
            FieldKind::Positional => {
                quote! {
                    __redacted.extend(#field_name.slot.into_iter());
                }
            }
            FieldKind::SubCommand => {
                quote! {
                    if let Some(__subcommand_args) = #field_name {
                        __redacted.extend(__subcommand_args.into_iter());
                    }
                }
            }
            FieldKind::Flatten => unreachable!(),
        }
    })
}

/// `arg_to_slot.push((...))` statements for a single option/switch field, mapping its
/// short/long/alias forms to the runtime `__slot` counter in a flatten contribution's
/// `append`.
fn flag_str_to_output_table_map_entries_for_contribution<'a>(
    field: &'a StructField<'a>,
) -> TokenStream {
    let long_name = field.long_name.as_ref().expect("option/switch has a long name");
    let mut entries = Vec::new();
    if let Some(short) = &field.attrs.short {
        let short = format!("-{}", short.value());
        entries.push(quote! { arg_to_slot.push((#short, __slot)); });
    }
    entries.push(quote! { arg_to_slot.push((#long_name, __slot)); });
    for alias in &field.attrs.aliases {
        let alias = format!("--{}", alias.value());
        entries.push(quote! { arg_to_slot.push((#alias, __slot)); });
    }
    quote! { #( #entries )* }
}

/// Entries of tokens like `("--some-flag-key", 5)` that map from a flag key string
/// to an index in the output table.
fn flag_str_to_output_table_map_entries<'a>(fields: &'a [StructField<'a>]) -> Vec<TokenStream> {
    let mut flag_str_to_output_table_map = vec![];
    for (i, (field, long_name)) in fields
        .iter()
        .filter_map(|field| field.long_name.as_ref().map(|long_name| (field, long_name)))
        .enumerate()
    {
        if let Some(short) = &field.attrs.short {
            let short = format!("-{}", short.value());
            flag_str_to_output_table_map.push(quote! { (#short, #i) });
        }

        flag_str_to_output_table_map.push(quote! { (#long_name, #i) });

        for alias in &field.attrs.aliases {
            let alias = format!("--{}", alias.value());
            flag_str_to_output_table_map.push(quote! { (#alias, #i) });
        }
    }
    flag_str_to_output_table_map
}

/// The long, short, and alias forms of every option/switch declared as
/// `global`, used to recognize global options after a subcommand.
fn global_options_entries(fields: &[StructField<'_>]) -> Vec<TokenStream> {
    let mut entries = vec![];
    for field in fields {
        if !field.attrs.global {
            continue;
        }
        if let Some(long_name) = &field.long_name {
            entries.push(quote! { #long_name });
        }
        if let Some(short) = &field.attrs.short {
            let short = format!("-{}", short.value());
            entries.push(quote! { #short });
        }
        for alias in &field.attrs.aliases {
            let alias = format!("--{}", alias.value());
            entries.push(quote! { #alias });
        }
    }
    entries
}

/// Conflict pairs among options/switches, resolved to their slot indices in the
/// flag output table plus their canonical `--long` display names. Each entry is
/// `(pos_a, name_a, pos_b, name_b)`, meaning option `a` at slot `pos_a` and
/// option `b` at slot `pos_b` are mutually exclusive.
fn conflicts_entries(
    errors: &Errors,
    fields: &[StructField<'_>],
) -> Vec<(usize, String, usize, String)> {
    // Option/switch fields, in the same order as the flag output table.
    let option_fields: Vec<&StructField<'_>> = fields
        .iter()
        .filter(|field| matches!(field.kind, FieldKind::Option | FieldKind::Switch))
        .collect();

    // Map from long name (without `--`) to the slot index of the field.
    let mut index_by_long: HashMap<String, usize> = HashMap::new();
    for (i, field) in option_fields.iter().enumerate() {
        if let Some(long_name) = &field.long_name {
            index_by_long.insert(long_name.trim_start_matches("--").to_owned(), i);
        }
    }

    let mut pairs: Vec<(usize, String, usize, String)> = Vec::new();
    for (i, field) in option_fields.iter().enumerate() {
        for conflict in &field.attrs.conflicts_with {
            let ref_name = conflict.value();
            match index_by_long.get(&ref_name) {
                Some(&j) if i != j => {
                    let name_i = field.long_name.as_ref().unwrap().clone();
                    let name_j = option_fields[j].long_name.as_ref().unwrap().clone();
                    let already = pairs
                        .iter()
                        .any(|&(pa, _, pb, _)| (pa == i && pb == j) || (pa == j && pb == i));
                    if !already {
                        pairs.push((i, name_i, j, name_j));
                    }
                }
                _ => {
                    errors.err(
                        conflict,
                        &format!("`conflicts_with` references unknown option `{ref_name}`"),
                    );
                }
            }
        }
    }
    pairs
}

/// Token entries of the form `(pos_a, "--name-a", pos_b, "--name-b")` passed to
/// `argy::ParseStructOptions` so the parser can reject mutually exclusive
/// options. The `seen` array indexes into the same slot table.
fn conflicts_entries_tokens(fields: &[StructField<'_>], errors: &Errors) -> Vec<TokenStream> {
    conflicts_entries(errors, fields)
        .into_iter()
        .map(|(pa, na, pb, nb)| {
            let na = syn::LitStr::new(&na, Span::call_site());
            let nb = syn::LitStr::new(&nb, Span::call_site());
            quote! { (#pa, #na, #pb, #nb) }
        })
        .collect()
}

/// Compute `requires` relationships: for each option/switch field with a
/// `requires` attribute, an entry `(pos_a, name_a, pos_b, name_b)` meaning
/// "if `pos_a` is seen, `pos_b` must also be seen". Also validates that every
/// `requires` reference names an existing option/switch long name.
fn requires_entries(
    errors: &Errors,
    fields: &[StructField<'_>],
) -> Vec<(usize, String, usize, String)> {
    // Option/switch fields, in the same order as the flag output table.
    let option_fields: Vec<&StructField<'_>> = fields
        .iter()
        .filter(|field| matches!(field.kind, FieldKind::Option | FieldKind::Switch))
        .collect();

    // Map from long name (without `--`) to the slot index of the field.
    let mut index_by_long: HashMap<String, usize> = HashMap::new();
    for (i, field) in option_fields.iter().enumerate() {
        if let Some(long_name) = &field.long_name {
            index_by_long.insert(long_name.trim_start_matches("--").to_owned(), i);
        }
    }

    let mut pairs: Vec<(usize, String, usize, String)> = Vec::new();
    for (i, field) in option_fields.iter().enumerate() {
        for req in &field.attrs.requires {
            let ref_name = req.value();
            match index_by_long.get(&ref_name) {
                Some(&j) if i != j => {
                    let name_i = field.long_name.as_ref().unwrap().clone();
                    let name_j = option_fields[j].long_name.as_ref().unwrap().clone();
                    pairs.push((i, name_i, j, name_j));
                }
                _ => {
                    errors.err(req, &format!("`requires` references unknown option `{ref_name}`"));
                }
            }
        }
    }
    pairs
}

/// One `if` statement per `requires` relationship for the post-parse check: if
/// slot `pos_a` was seen but slot `pos_b` was not, report `--name-b` as a
/// missing required option. `mri` is the `MissingRequirements` local ident.
/// The `seen` array indexes into the same slot table.
fn requires_check_tokens(
    fields: &[StructField<'_>],
    errors: &Errors,
    mri: &syn::Ident,
) -> Vec<TokenStream> {
    requires_entries(errors, fields)
        .into_iter()
        .map(|(pa, _, pb, nb)| {
            let pa = syn::Index::from(pa);
            let pb = syn::Index::from(pb);
            let nb = syn::LitStr::new(&nb, Span::call_site());
            quote! {
                if __seen[#pa] && !__seen[#pb] {
                    #mri.missing_option(#nb);
                }
            }
        })
        .collect()
}

/// For each non-optional field, add an entry to the `argy::MissingRequirements`.
fn append_missing_requirements<'a>(
    // missing_requirements_ident
    mri: &syn::Ident,
    fields: &'a [StructField<'a>],
) -> impl Iterator<Item = TokenStream> + 'a {
    let mri = mri.clone();
    fields
        .iter()
        .filter(|f| {
            f.optionality.is_required()
                || (f.kind == FieldKind::Positional && f.attrs.greedy.is_some() && f.attrs.required)
        })
        .map(move |field| {
            let field_name = field.name;
            match field.kind {
                FieldKind::Switch => unreachable!("switches are always optional"),
                FieldKind::Positional => {
                    let name = field.positional_arg_name();
                    if field.attrs.greedy.is_some() && field.attrs.required {
                        quote! {
                            if #field_name.slot.is_empty() {
                                #mri.missing_positional_arg(#name)
                            }
                        }
                    } else {
                        quote! {
                            if #field_name.slot.is_none() {
                                #mri.missing_positional_arg(#name)
                            }
                        }
                    }
                }
                FieldKind::Option => {
                    let name = field.long_name.as_ref().expect("options always have a long name");
                    quote! {
                        if #field_name.slot.is_none() {
                            #mri.missing_option(#name)
                        }
                    }
                }
                FieldKind::SubCommand => {
                    let ty = field.ty_without_wrapper;
                    quote! {
                        if #field_name.is_none() {
                            #mri.missing_subcommands(
                                <#ty as argy::SubCommands>::COMMANDS
                                    .iter()
                                    .cloned()
                                    .chain(
                                        <#ty as argy::SubCommands>::dynamic_commands()
                                            .iter()
                                            .copied()
                                    ),
                            )
                        }
                    }
                }
                FieldKind::Flatten => unreachable!(),
            }
        })
}

/// Require that a type can be a `switch`.
/// Throws an error for all types except booleans and integers
fn ty_expect_switch(errors: &Errors, ty: &syn::Type) -> bool {
    fn ty_can_be_switch(ty: &syn::Type) -> bool {
        if let syn::Type::Path(path) = ty {
            if path.qself.is_some() {
                return false;
            }
            if path.path.segments.len() != 1 {
                return false;
            }
            let ident = &path.path.segments[0].ident;
            // `Option<bool>` can be used as a `switch`.
            if ident == "Option" {
                if let PathArguments::AngleBracketed(args) = &path.path.segments[0].arguments {
                    if let GenericArgument::Type(Type::Path(p)) = &args.args[0] {
                        if p.path.segments[0].ident == "bool" {
                            return true;
                        }
                    }
                }
            }
            ["bool", "u8", "u16", "u32", "u64", "u128", "i8", "i16", "i32", "i64", "i128"]
                .iter()
                .any(|path| ident == path)
        } else {
            false
        }
    }

    let res = ty_can_be_switch(ty);
    if !res {
        errors.err(ty, "switches must be of type `bool`, `Option<bool>`, or integer type");
    }
    res
}

/// Returns `true` if the type is exactly `Option<bool>` (used to detect an
/// optional-value switch whose usage should show `[=<bool>]`).
pub(crate) fn ty_is_option_bool(ty: &syn::Type) -> bool {
    if let syn::Type::Path(path) = ty {
        if path.qself.is_some() || path.path.segments.len() != 1 {
            return false;
        }
        if path.path.segments[0].ident != "Option" {
            return false;
        }
        let syn::PathArguments::AngleBracketed(args) = &path.path.segments[0].arguments else {
            return false;
        };
        let Some(syn::GenericArgument::Type(syn::Type::Path(p))) = args.args.first() else {
            return false;
        };
        p.path.segments.len() == 1 && p.path.segments[0].ident == "bool"
    } else {
        false
    }
}

/// Returns `Some(T)` if a type is `wrapper_name<T>` for any `wrapper_name` in `wrapper_names`.
fn ty_inner<'a>(wrapper_names: &[&str], ty: &'a syn::Type) -> Option<&'a syn::Type> {
    if let syn::Type::Path(path) = ty {
        if path.qself.is_some() {
            return None;
        }
        // Since we only check the last path segment, it isn't necessarily the case that
        // we're referring to `std::vec::Vec` or `std::option::Option`, but there isn't
        // a fool proof way to check these since name resolution happens after macro expansion,
        // so this is likely "good enough" (so long as people don't have their own types called
        // `Option` or `Vec` that take one generic parameter they're looking to parse).
        let last_segment = path.path.segments.last()?;
        if !wrapper_names.iter().any(|name| last_segment.ident == *name) {
            return None;
        }
        if let syn::PathArguments::AngleBracketed(gen_args) = &last_segment.arguments {
            let generic_arg = gen_args.args.first()?;
            if let syn::GenericArgument::Type(ty) = &generic_arg {
                return Some(ty);
            }
        }
    }
    None
}

/// Implements `FromArgs` and `SubCommands` for a `#![derive(FromArgs)]` enum.
// Too many lines: this helper builds a large generated token stream.
#[allow(clippy::too_many_lines)]
fn impl_from_args_enum(
    errors: &Errors,
    name: &syn::Ident,
    type_attrs: &TypeAttrs,
    generic_args: &syn::Generics,
    de: &syn::DataEnum,
) -> TokenStream {
    parse_attrs::check_enum_type_attrs(errors, type_attrs, de.enum_token.span);

    // An enum variant like `<name>(<ty>)`
    #[allow(clippy::items_after_statements)] // Local helper struct used by the generated impl below.
    struct SubCommandVariant<'a> {
        name: &'a syn::Ident,
        ty: &'a syn::Type,
    }

    let mut dynamic_type_and_variant = None;

    let variants: Vec<SubCommandVariant<'_>> = de
        .variants
        .iter()
        .filter_map(|variant| {
            let name = &variant.ident;
            let ty = enum_only_single_field_unnamed_variants(errors, &variant.fields)?;
            if parse_attrs::VariantAttrs::parse(errors, variant).is_dynamic.is_some() {
                if dynamic_type_and_variant.is_some() {
                    errors.err(variant, "Only one variant can have the `dynamic` attribute");
                }
                dynamic_type_and_variant = Some((ty, name));
                None
            } else {
                Some(SubCommandVariant { name, ty })
            }
        })
        .collect();

    let name_repeating = std::iter::repeat(name.clone());
    let variant_ty = variants.iter().map(|x| x.ty).collect::<Vec<_>>();
    let variant_names = variants.iter().map(|x| x.name).collect::<Vec<_>>();
    let dynamic_from_args =
        dynamic_type_and_variant.as_ref().map(|(dynamic_type, dynamic_variant)| {
            quote! {
                if let Some(result) = <#dynamic_type as argy::DynamicSubCommand>::try_from_args(
                    command_name, args) {
                    return result.map(#name::#dynamic_variant);
                }
            }
        });
    let dynamic_redact_arg_values = dynamic_type_and_variant.as_ref().map(|(dynamic_type, _)| {
        quote! {
            if let Some(result) = <#dynamic_type as argy::DynamicSubCommand>::try_redact_arg_values(
                command_name, args) {
                return result;
            }
        }
    });
    let dynamic_commands = dynamic_type_and_variant.as_ref().map(|(dynamic_type, _)| {
        quote! {
            fn dynamic_commands() -> &'static [&'static argy::CommandInfo] {
                <#dynamic_type as argy::DynamicSubCommand>::commands()
            }
        }
    });

    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();
    quote! {
        impl #impl_generics argy::FromArgs for #name #ty_generics #where_clause {
            fn from_args(command_name: &[&str], args: &[&str])
                -> std::result::Result<Self, argy::EarlyExit>
            {
                let subcommand_name = if let Some(subcommand_name) = command_name.last() {
                    *subcommand_name
                } else {
                    return ::core::result::Result::Err(argy::EarlyExit::from("no subcommand name".to_owned()));
                };

                #(
                    if subcommand_name == <#variant_ty as argy::SubCommand>::COMMAND.name
                        || (*<#variant_ty as argy::SubCommand>::COMMAND.short != '\0'
                            && subcommand_name.len() == 1
                            && subcommand_name.starts_with(*<#variant_ty as argy::SubCommand>::COMMAND.short))
                    {
                        return ::core::result::Result::Ok(#name_repeating::#variant_names(
                            <#variant_ty as argy::FromArgs>::from_args(command_name, args)?
                        ));
                    }
                )*

                #dynamic_from_args

                ::core::result::Result::Err(argy::EarlyExit::from("no subcommand matched".to_owned()))
            }

            fn redact_arg_values(command_name: &[&str], args: &[&str]) -> std::result::Result<Vec<String>, argy::EarlyExit> {
                let subcommand_name = if let Some(subcommand_name) = command_name.last() {
                    *subcommand_name
                } else {
                    return ::core::result::Result::Err(argy::EarlyExit::from("no subcommand name".to_owned()));
                };

                #(
                    if subcommand_name == <#variant_ty as argy::SubCommand>::COMMAND.name
                        || (*<#variant_ty as argy::SubCommand>::COMMAND.short != '\0'
                            && subcommand_name.len() == 1
                            && subcommand_name.starts_with(*<#variant_ty as argy::SubCommand>::COMMAND.short))
                    {
                        return <#variant_ty as argy::FromArgs>::redact_arg_values(command_name, args);
                    }
                )*

                #dynamic_redact_arg_values

                ::core::result::Result::Err(argy::EarlyExit::from("no subcommand matched".to_owned()))
            }
        }

        impl #impl_generics argy::SubCommands for #name #ty_generics #where_clause {
            const COMMANDS: &'static [&'static argy::CommandInfo] = &[#(
                <#variant_ty as argy::SubCommand>::COMMAND,
            )*];

            #dynamic_commands
        }
    }
}

/// Returns `Some(Bar)` if the field is a single-field unnamed variant like `Foo(Bar)`.
/// Otherwise, generates an error.
fn enum_only_single_field_unnamed_variants<'a>(
    errors: &Errors,
    variant_fields: &'a syn::Fields,
) -> Option<&'a syn::Type> {
    macro_rules! with_enum_suggestion {
        ($help_text:literal) => {
            concat!(
                $help_text,
                "\nInstead, use a variant with a single unnamed field for each subcommand:\n",
                "    enum MyCommandEnum {\n",
                "        SubCommandOne(SubCommandOne),\n",
                "        SubCommandTwo(SubCommandTwo),\n",
                "    }",
            )
        };
    }

    match variant_fields {
        syn::Fields::Named(fields) => {
            errors.err(
                fields,
                with_enum_suggestion!(
                    "`#![derive(FromArgs)]` `enum`s do not support variants with named fields."
                ),
            );
            None
        }
        syn::Fields::Unit => {
            errors.err(
                variant_fields,
                with_enum_suggestion!(
                    "`#![derive(FromArgs)]` does not support `enum`s with no variants."
                ),
            );
            None
        }
        syn::Fields::Unnamed(fields) => {
            if fields.unnamed.len() == 1 {
                // `unwrap` is okay because of the length check above.
                let first_field = fields.unnamed.first().unwrap();
                Some(&first_field.ty)
            } else {
                errors.err(
                    fields,
                    with_enum_suggestion!(
                        "`#![derive(FromArgs)]` `enum` variants must only contain one field."
                    ),
                );
                None
            }
        }
    }
}

/// Implements `FromArgValue` for a `#![derive(FromArgValue)]` enum (a choice enum).
fn impl_from_arg_value_enum(
    errors: &Errors,
    name: &syn::Ident,
    generic_args: &syn::Generics,
    de: &syn::DataEnum,
) -> TokenStream {
    // An enum variant like `<name>`
    struct ChoiceVariant<'a> {
        ident: &'a syn::Ident,
        name: syn::LitStr,
        aliases: Vec<syn::LitStr>,
    }

    let variants: Vec<ChoiceVariant<'_>> = de
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;
            choice_enum_only_fieldless_variant(errors, &variant.fields);
            let attrs = parse_attrs::ChoiceVariantAttrs::parse(errors, variant);
            let name = attrs.name_override.unwrap_or_else(|| {
                let name_str = pascal_to_snake_case(&format!("{ident}"));
                syn::LitStr::new(&name_str, ident.span())
            });
            ChoiceVariant { ident, name, aliases: attrs.aliases }
        })
        .collect();

    if variants.is_empty() {
        errors.err(&de.variants, "Choice enums must have at least one variant");
    }

    let name_repeating = std::iter::repeat(name.clone());
    let variant_idents = variants.iter().map(|x| x.ident);
    let variant_names = variants.iter().map(|x| &x.name).collect::<Vec<_>>();
    // A `|`-separated pattern per variant that accepts the canonical name and any aliases.
    let variant_match_patterns = variants.iter().map(|variant| {
        let variant_name = &variant.name;
        let mut pattern = quote! { #variant_name };
        for alias in &variant.aliases {
            pattern = quote! { #pattern | #alias };
        }
        pattern
    });
    let err_literal = {
        let mut err = "expected ".to_string();
        for (i, name) in variant_names.iter().enumerate() {
            if i == 0 {
            } else if i == variant_names.len() - 1 {
                err.push_str(" or ");
            } else {
                err.push_str(", ");
            }
            let _ = write!(err, "{:?}", name.value());
        }
        LitStr::new(&err, name.span())
    };
    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();
    quote! {
        impl #impl_generics argy::FromArgValue for #name #ty_generics #where_clause {
            fn from_arg_value(value: &str)
                -> ::core::result::Result<Self, String>
            {
                ::core::result::Result::Ok(match value {
                    #(
                        #variant_match_patterns => #name_repeating::#variant_idents,
                    )*
                    _ => {
                        return ::core::result::Result::Err(#err_literal.to_owned())
                    }
                })
            }
        }
    }
}

/// Generates an error if the variant is not a field-less variant like `Foo`.
fn choice_enum_only_fieldless_variant(errors: &Errors, variant_fields: &syn::Fields) {
    match variant_fields {
        syn::Fields::Unit => {}
        _ => {
            errors.err(
                variant_fields,
                "Choice `enum`s tagged with `#![derive(FromArgValue)]` do not support variants with associated data.",
            );
        }
    }
}

fn pascal_to_snake_case(camel: &str) -> String {
    let mut out = String::with_capacity(camel.len() + 8);
    for (i, c) in camel.chars().enumerate() {
        if c.is_uppercase() {
            if i != 0 {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// Convert a `PascalCase` ident to its string form using `sep` between words
/// (e.g. `FooBar` -> `foo-bar` for `"-"`, `foo_bar` for `"_"`).
fn pascal_to_case(camel: &str, sep: &str) -> String {
    pascal_to_snake_case(camel).replace('_', sep)
}
