extern crate proc_macro;

use syn::{parse_macro_input, punctuated::Punctuated, ItemStruct, Meta, Token};

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;

#[proc_macro_attribute]
pub fn prpc_serde_bytes(_attr: TokenStream, item: TokenStream) -> TokenStream {
    patch(parse_macro_input!(item)).into()
}

pub(crate) fn patch(item: TokenStream2) -> TokenStream2 {
    match patch_or_err(item) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    }
}

fn has_attr(attrs: &[syn::Attribute], name: &str, field: &str) -> bool {
    for attr in attrs.iter() {
        if attr.path().is_ident(name) {
            let Ok(nested) = attr.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            else {
                continue;
            };
            for meta in nested.iter() {
                if meta.path().is_ident(field) {
                    return true;
                }
            }
        }
    }
    false
}

fn patch_or_err(input: TokenStream2) -> syn::Result<TokenStream2> {
    let Ok(mut input) = syn::parse2::<ItemStruct>(input.clone()) else {
        return Ok(input);
    };
    for field in input.fields.iter_mut() {
        if !has_attr(&field.attrs, "serde", "with") && has_attr(&field.attrs, "prost", "bytes") {
            if has_attr(&field.attrs, "prost", "optional") {
                field
                    .attrs
                    .push(syn::parse_quote!(#[serde(with = "::prpc::serde_helpers::option_bytes_as_hex_str")]));
            } else if has_attr(&field.attrs, "prost", "repeated") {
                field
                    .attrs
                    .push(syn::parse_quote!(#[serde(with = "::prpc::serde_helpers::vec_bytes_as_hex_str")]));
            } else {
                field.attrs.push(
                    syn::parse_quote!(#[serde(with = "::prpc::serde_helpers::bytes_as_hex_str")]),
                );
            }
        }
        if !has_attr(&field.attrs, "serde", "default")
            && has_attr(&field.attrs, "prost", "repeated")
        {
            field.attrs.push(syn::parse_quote!(#[serde(default)]));
        }
    }

    Ok(syn::parse_quote! {
        #input
    })
}

#[cfg(test)]
mod tests;
