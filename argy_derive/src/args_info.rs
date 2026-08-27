// Copyright (c) 2023 Google LLC All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use crate::{
    enum_only_single_field_unnamed_variants,
    errors::Errors,
    help::require_description,
    parse_attrs::{check_enum_type_attrs, FieldAttrs, FieldKind, TypeAttrs, VariantAttrs},
    Optionality, StructField,
};
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::LitStr;

/// Implement the derive macro for `ArgsInfo`.
pub fn impl_args_info(input: &syn::DeriveInput) -> TokenStream {
    let errors = &Errors::default();

    // parse the types
    let type_attrs = &TypeAttrs::parse(errors, input);

    // Based on the type generate the appropriate code.
    let mut output_tokens = match &input.data {
        syn::Data::Struct(ds) => {
            impl_arg_info_struct(errors, &input.ident, type_attrs, &input.generics, ds)
        }
        syn::Data::Enum(de) => {
            impl_arg_info_enum(errors, &input.ident, type_attrs, &input.generics, de)
        }
        syn::Data::Union(_) => {
            errors.err(input, "`#[derive(ArgsInfo)]` cannot be applied to unions");
            TokenStream::new()
        }
    };
    errors.to_tokens(&mut output_tokens);
    output_tokens
}

/// Implement the `ArgsInfo` trait for a struct annotated with argy attributes.
fn impl_arg_info_struct(
    errors: &Errors,
    name: &syn::Ident,
    type_attrs: &TypeAttrs,
    generic_args: &syn::Generics,
    ds: &syn::DataStruct,
) -> TokenStream {
    // Collect the fields, skipping fields that are not supported.
    let fields = match &ds.fields {
        syn::Fields::Named(fields) => fields,
        syn::Fields::Unnamed(_) => {
            errors.err(
                &ds.struct_token,
                "`#![derive(ArgsInfo)]` is not currently supported on tuple structs",
            );
            return TokenStream::new();
        }
        syn::Fields::Unit => {
            errors.err(&ds.struct_token, "#![derive(ArgsInfo)]` cannot be applied to unit structs");
            return TokenStream::new();
        }
    };

    // Map the fields into StructField objects.
    let fields: Vec<_> = fields
        .named
        .iter()
        .filter_map(|field| {
            let attrs = FieldAttrs::parse(errors, field);
            StructField::new(errors, field, attrs)
        })
        .collect();

    let impl_span = Span::call_site();

    // Generate the implementation of `get_args_info()` for this struct.
    let args_info = impl_args_info_data(name, errors, type_attrs, &fields);

    // Split out the generics info for the impl declaration.
    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();

    quote_spanned! { impl_span =>
        #[automatically_derived]
        impl #impl_generics argy::ArgsInfo for #name #ty_generics #where_clause {
           fn get_args_info() -> argy::CommandInfoWithArgs {
            #args_info
           }
        }
    }
}

/// Implement `ArgsInfo` for an enum. The enum is a collection of subcommands.
fn impl_arg_info_enum(
    errors: &Errors,
    name: &syn::Ident,
    type_attrs: &TypeAttrs,
    generic_args: &syn::Generics,
    de: &syn::DataEnum,
) -> TokenStream {
    // Validate the enum is OK for argy.
    check_enum_type_attrs(errors, type_attrs, de.enum_token.span);

    // Ensure that `#[argy(subcommand)]` is present.
    if type_attrs.is_subcommand.is_none() {
        errors.err_span(
            de.enum_token.span,
            concat!(
                "`#![derive(ArgsInfo)]` on `enum`s can only be used to enumerate subcommands.\n",
                "Consider adding `#[argy(subcommand)]` to the `enum` declaration.",
            ),
        );
    }

    // One of the variants can be annotated as providing dynamic subcommands.
    // We treat this differently since we need to call a function at runtime
    // to determine the subcommands provided.
    let mut dynamic_type_and_variant = None;

    // An enum variant like `<name>(<ty>)`. This is used to collect
    // the type of the variant for each subcommand.
    #[allow(clippy::items_after_statements)] // helper struct used after local statements above
    struct ArgInfoVariant<'a> {
        ty: &'a syn::Type,
    }

    let variants: Vec<ArgInfoVariant<'_>> = de
        .variants
        .iter()
        .filter_map(|variant| {
            let name = &variant.ident;
            let ty = enum_only_single_field_unnamed_variants(errors, &variant.fields)?;
            if VariantAttrs::parse(errors, variant).is_dynamic.is_some() {
                if dynamic_type_and_variant.is_some() {
                    errors.err(variant, "Only one variant can have the `dynamic` attribute");
                }
                dynamic_type_and_variant = Some((ty, name));
                None
            } else {
                Some(ArgInfoVariant { ty })
            }
        })
        .collect();

    let dynamic_subcommands = if let Some((dynamic_type, _)) = dynamic_type_and_variant {
        quote! {
            <#dynamic_type as argy::DynamicSubCommand>::commands().iter()
            .map(|s|
         SubCommandInfo {
                name: s.name,
                command: CommandInfoWithArgs {
                    name: s.name,
                    short: s.short,
                    description: s.description,
                    hidden: s.hidden,
                    aliases: s.aliases,
                    ..Default::default()
                }
            }).collect()
        }
    } else {
        quote! { vec![]}
    };

    let variant_ty_info = variants.iter().map(|t| {
        let ty = t.ty;
        quote!(
            argy::SubCommandInfo {
                name: #ty::get_args_info().name,
                command: #ty::get_args_info()
            }
        )
    });

    let cmd_name =
        type_attrs.name.as_ref().map_or_else(|| LitStr::new("", Span::call_site()), Clone::clone);

    let short_name =
        type_attrs.short.as_ref().map_or_else(|| quote! { &'\0' }, |id| quote! { &#id });

    let (impl_generics, ty_generics, where_clause) = generic_args.split_for_impl();

    quote! {
        #[automatically_derived]
        impl #impl_generics argy::ArgsInfo for #name #ty_generics #where_clause {
           fn get_args_info() -> argy::CommandInfoWithArgs {

            let mut the_subcommands = vec![#(#variant_ty_info),*];
            let mut dynamic_commands = #dynamic_subcommands;

            the_subcommands.append(&mut dynamic_commands);


            argy::CommandInfoWithArgs {
                name: #cmd_name,
                short: #short_name,
               /// A short description of the command's functionality.
                description: " enum of subcommands",
                commands: the_subcommands,
                ..Default::default()
               }
           } // end of get_args_ifo
        }  // end of impl ArgsInfo
    }
}

#[allow(clippy::too_many_lines)] // large generated-code builder; refactor would obscure the layout
fn impl_args_info_data<'a>(
    name: &proc_macro2::Ident,
    errors: &Errors,
    type_attrs: &TypeAttrs,
    fields: &'a [StructField<'a>],
) -> TokenStream {
    let mut subcommands_iter =
        fields.iter().filter(|field| field.kind == FieldKind::SubCommand).fuse();

    let subcommand: Option<&StructField<'_>> = subcommands_iter.next();
    for dup_subcommand in subcommands_iter {
        errors.duplicate_attrs("subcommand", subcommand.unwrap().field, dup_subcommand.field);
    }

    let impl_span = Span::call_site();

    let mut positionals = vec![];
    let mut flags = vec![];

    // Add the implicit --help flag
    flags.push(quote! {
        argy::FlagInfo {
            short: None,
            long: "--help",
            description: "display usage information",
            optionality: argy::Optionality::Optional,
            kind: argy::FlagInfoKind::Switch,
            hidden: false,
            aliases: &[],
        }
    });

    for field in fields {
        let optionality = match field.optionality {
            Optionality::None => quote! { argy::Optionality::Required },
            Optionality::Defaulted(_) | Optionality::Optional => {
                quote! { argy::Optionality::Optional }
            }
            Optionality::Repeating | Optionality::DefaultedRepeating(_)
                if field.attrs.greedy.is_some() =>
            {
                quote! { argy::Optionality::Greedy }
            }
            Optionality::Repeating | Optionality::DefaultedRepeating(_) => {
                quote! { argy::Optionality::Repeating }
            }
        };

        match field.kind {
            FieldKind::Positional => {
                let name = field.positional_arg_name();

                let description = field
                    .attrs
                    .description
                    .as_ref()
                    .map_or_else(String::new, |desc| desc.content.value().trim().to_owned());
                let hidden = field.attrs.hidden_help;

                positionals.push(quote! {
                    argy::PositionalInfo {
                        name: #name,
                        description: #description,
                        optionality: #optionality,
                        hidden: #hidden,
                    }
                });
            }
            FieldKind::Switch | FieldKind::Option => {
                let short = field
                    .attrs
                    .short
                    .as_ref()
                    .map_or_else(|| quote! { None }, |short| quote! { Some(#short) });

                let long = field.long_name.as_ref().expect("missing long name for option");

                let description = require_description(
                    errors,
                    field.name.span(),
                    &field.attrs.description,
                    "field",
                );

                let kind = if field.kind == FieldKind::Switch {
                    quote! {
                        argy::FlagInfoKind::Switch
                    }
                } else {
                    let arg_name = field.attrs.arg_name.as_ref().map_or_else(
                        || {
                            let arg_name = long.trim_start_matches("--");
                            quote! { #arg_name }
                        },
                        |arg_name| quote! { #arg_name },
                    );

                    quote! {
                        argy::FlagInfoKind::Option {
                            arg_name: #arg_name,
                        }
                    }
                };

                let hidden = field.attrs.hidden_help;

                let flag_aliases = field.attrs.aliases.iter().map(|lit| {
                    let s = format!("--{}", lit.value());
                    syn::LitStr::new(&s, lit.span())
                });
                let flag_aliases = quote! { &[#( #flag_aliases, )*] };

                flags.push(quote! {
                    argy::FlagInfo {
                        short: #short,
                        long: #long,
                        description: #description,
                        optionality: #optionality,
                        kind: #kind,
                        hidden: #hidden,
                        aliases: #flag_aliases,
                    }
                });
            }
            FieldKind::SubCommand | FieldKind::Flatten => {}
        }
    }

    let empty_str = syn::LitStr::new("", Span::call_site());
    let type_name = LitStr::new(&name.to_string(), Span::call_site());
    let subcommand_name = if type_attrs.is_subcommand.is_some() {
        type_attrs.name.as_ref().unwrap_or_else(|| {
            errors.err(name, "`#[argy(name = \"...\")]` attribute is required for subcommands");
            &empty_str
        })
    } else {
        &type_name
    };

    let subcommand = subcommand.map_or_else(
        || quote! { vec![] },
        |subcommand| {
            let subcommand_ty = subcommand.ty_without_wrapper;
            quote! {
                #subcommand_ty::get_subcommands()
            }
        },
    );

    let description =
        require_description(errors, Span::call_site(), &type_attrs.description, "type");
    let examples = type_attrs.examples.iter().map(|e| quote! { #e });
    let notes = type_attrs.notes.iter().map(|e| quote! { #e });

    let error_codes = type_attrs.error_codes.iter().map(|(code, text)| {
        quote! { argy::ErrorCodeInfo{code:#code, description: #text} }
    });

    let short_name =
        type_attrs.short.as_ref().map_or_else(|| quote! { &'\0' }, |id| quote! { &#id });

    let hidden = type_attrs.hidden;
    let aliases = type_attrs.aliases.iter().map(|lit| syn::LitStr::new(&lit.value(), lit.span()));
    let aliases = quote! { &[#( #aliases, )*] };

    quote_spanned! { impl_span =>
        argy::CommandInfoWithArgs {
            name: #subcommand_name,
            short: #short_name,
            description: #description,
            examples: &[#( #examples, )*],
            notes: &[#( #notes, )*],
            positionals: &[#( #positionals, )*],
            flags: &[#( #flags, )*],
            commands: #subcommand,
            error_codes: &[#( #error_codes, )*],
            hidden: #hidden,
            aliases: #aliases,
        }
    }
}
