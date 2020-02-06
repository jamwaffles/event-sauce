use crate::derive_struct::{
    derive_entity, derive_event_create, derive_event_delete, derive_event_update,
};
use crate::_PROC_MACRO_NAME;
use proc_macro2::{Ident, Span, TokenStream, TokenTree};
use quote::__rt::TokenTree::Group;
use std::string::ToString;
use syn::{Attribute, Data, DataStruct, DeriveInput, Lit, Meta};

pub struct StructInfo {
    // pub entity_type: Ident,
    pub target_model: TokenStream,
}

impl StructInfo {
    pub fn new(parsed: &syn::DeriveInput, _struct_body: &DataStruct) -> Self {
        let target_model: TokenStream = parsed
            .attrs
            .iter()
            // Find "event_store" attribute, ignoring other attrs and comments
            .find(|attr| attr.path.is_ident("event_store"))
            .and_then(|attr| {
                attr.tokens
                    .clone()
                    .into_iter()
                    .filter_map(|tt| match tt {
                        Group(g) => Some(g.stream()),
                        _ => None,
                    })
                    .next()
            })
            .expect("Attribute must be provided a target model struct");

        Self { target_model }
    }
}

// Left for posterity - figuring this out was hard work!
pub fn _get_attribute_ident(input: &Vec<Attribute>, attribute_name: &'static str) -> Option<Ident> {
    _get_attribute(input, attribute_name).map(|attribute_value| {
        Ident::new(
            attribute_value.to_string().trim_matches('"').into(),
            Span::call_site(),
        )
    })
}

// Left for posterity - figuring this out was hard work!
pub fn _get_attribute(input: &Vec<Attribute>, attribute_name: &'static str) -> Option<TokenTree> {
    let ident_match = Ident::new(attribute_name, Span::call_site());

    input
        .iter()
        .filter_map(|attr| {
            // Look through all attribute annotations
            attr.path
                .segments
                .iter()
                .find(|segment| segment.ident.to_string() == _PROC_MACRO_NAME)
                .map(|_| {
                    // Find attribute triples like `namespace = "something"`
                    attr.clone().tokens.into_iter().filter(|tt| match tt {
                        Group(_) => true,
                        _ => false,
                    })
                })
                .and_then(|mut groups| {
                    groups.find_map(|tt| {
                        match tt {
                            Group(g) => g
                                .stream()
                                .into_iter()
                                // Look for the identifier we want
                                .skip_while(|item| match item {
                                    TokenTree::Ident(ref ident) if *ident == ident_match => false,
                                    _ => true,
                                })
                                // Once ident is found, skip ahead until its associated value is found
                                .skip_while(|item| match item {
                                    TokenTree::Literal(_) => false,
                                    _ => true,
                                })
                                .next(),
                            _ => None,
                        }
                    })
                })
        })
        .next()
}

// Parse `#[table_name = "the_name"]` into `String("the_name")`
pub fn diesel_table_name(input: &Vec<Attribute>) -> Option<String> {
    input.iter().find_map(|attr| {
        let meta = attr.parse_meta().ok()?;

        if meta.path().is_ident("table_name") {
            match meta {
                Meta::NameValue(kv) => match kv.lit {
                    Lit::Str(name) => Some(name.value()),
                    _ => None,
                },
                _ => None,
            }
        } else {
            None
        }
    })
}

pub fn expand_derive_event_create(parsed: &DeriveInput) -> TokenStream {
    match parsed.data {
        Data::Struct(ref body) => derive_event_create(&parsed, &body),
        _ => panic!("Event store can only be derived on structs"),
    }
}

pub fn expand_derive_event_update(parsed: &DeriveInput) -> TokenStream {
    match parsed.data {
        Data::Struct(ref body) => derive_event_update(&parsed, &body),
        _ => panic!("Event store can only be derived on structs"),
    }
}

pub fn expand_derive_event_delete(parsed: &DeriveInput) -> TokenStream {
    match parsed.data {
        Data::Struct(ref body) => derive_event_delete(&parsed, &body),
        _ => panic!("Event store can only be derived on structs"),
    }
}

pub fn expand_derive_entity(parsed: &DeriveInput) -> TokenStream {
    match parsed.data {
        Data::Struct(ref body) => derive_entity(&parsed, &body),
        _ => panic!("Event store can only be derived on structs"),
    }
}
