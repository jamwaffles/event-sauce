use proc_macro2::Span;
use quote::quote;
use syn::{
    punctuated::Punctuated, token::Comma, Attribute, Data, DataStruct, DeriveInput, Field, Fields,
    FieldsNamed, Lit, Meta, MetaNameValue, NestedMeta,
};

macro_rules! try_set {
    ($i:ident, $v:expr, $t:expr) => {
        match $i {
            None => $i = Some($v),
            Some(_) => return Err(syn::Error::new_spanned($t, "duplicate attribute")),
        }
    };
}

macro_rules! fail {
    ($t:expr, $m:expr) => {
        return Err(syn::Error::new_spanned($t, $m));
    };
}

struct EntityAttributes {
    entity_name: String,
}

fn parse_entity_attributes(input: &[Attribute]) -> syn::Result<EntityAttributes> {
    let mut entity_name = None;

    for attr in input {
        let meta = attr
            .parse_meta()
            .map_err(|e| syn::Error::new_spanned(attr, e))?;

        match meta {
            Meta::List(list) if list.path.is_ident("event_sauce") => {
                for value in list.nested.iter() {
                    match value {
                        NestedMeta::Meta(meta) => match meta {
                            Meta::NameValue(MetaNameValue {
                                path,
                                lit: Lit::Str(val),
                                ..
                            }) if path.is_ident("entity_name") => {
                                try_set!(entity_name, val.value(), value)
                            }

                            Meta::NameValue(MetaNameValue {
                                path,
                                lit: Lit::Str(_val),
                                ..
                            }) if !path.is_ident("entity_name") => fail!(
                                meta,
                                format!(
                                    "unrecognised attribute {:?}",
                                    path.get_ident()
                                        .map(|i| i.to_string())
                                        .expect("expected an attribute")
                                )
                            ),

                            u => fail!(u, "unexpected attribute"),
                        },
                        u => fail!(u, "unexpected attribute format"),
                    }
                }
            }
            _ => {}
        }
    }

    let entity_name = entity_name.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "Attribute entity_name is required, e.g. #[event_sauce(entity_name = \"users\")]",
        )
    })?;

    Ok(EntityAttributes { entity_name })
}

fn expand_derive_entity_struct(
    input: &DeriveInput,
    _fields: &Punctuated<Field, Comma>,
) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;

    let EntityAttributes { entity_name } = parse_entity_attributes(&input.attrs)?;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote!(
        impl #impl_generics event_sauce::Entity for #ident #ty_generics #where_clause {
            const ENTITY_TYPE: &'static str = #entity_name;
        }
    ))
}

pub fn expand_derive_entity(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => expand_derive_entity_struct(input, named),

        Data::Struct(DataStruct {
            fields: Fields::Unnamed(_),
            ..
        }) => Err(syn::Error::new_spanned(
            input,
            "tuple structs are not supported",
        )),

        Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => Err(syn::Error::new_spanned(
            input,
            "unit structs are not supported",
        )),

        Data::Enum(_) => Err(syn::Error::new_spanned(input, "enums are not supported")),

        Data::Union(_) => Err(syn::Error::new_spanned(input, "unions are not supported")),
    }
}
