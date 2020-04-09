
use proc_macro2::Span;
use quote::quote;
use syn::punctuated::Punctuated;
use syn::token::Comma;
use syn::DataStruct;
use syn::FieldsNamed;
use syn::Path;
use syn::{
    parse_quote, Attribute, Data, DeriveInput, Field, Fields, Lifetime, Lit, Meta, MetaNameValue,
    NestedMeta, Variant,
};

macro_rules! assert_attribute {
    ($e:expr, $err:expr, $input:expr) => {
        if !$e {
            return Err(syn::Error::new_spanned($input, $err));
        }
    };
}

/// Attempt to assign a value to a variable, failing if the variable is already populated.
///
/// Prevents attributes from being defined twice
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

enum BuilderType {
    Create,
    Update,
}

struct EventDataAttributes {
    entity: Path,
}

fn parse_event_data_attributes(input: &[Attribute]) -> syn::Result<EventDataAttributes> {
    let mut entity = None;

    for attr in input {
        let meta = attr
            .parse_meta()
            .map_err(|e| syn::Error::new_spanned(attr, e))?;

        match meta {
            Meta::List(list) if list.path.is_ident("event_sauce") => {
                for value in list.nested.iter() {
                    match value {
                        NestedMeta::Meta(meta) => match meta {
                            Meta::Path(path) => try_set!(entity, path.clone(), path),

                            u => fail!(u, "unexpected attribute"),
                        },
                        u => fail!(u, "unexpected attribute"),
                    }
                }
            }
            _ => {}
        }
    }

    let entity = entity.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            "Attribute entity is required, e.g. #[event_sauce(entity = User)",
        )
    })?;

    Ok(EventDataAttributes { entity })
}

fn expand_derive_event_data_struct(
    input: &DeriveInput,
    _fields: &Punctuated<Field, Comma>,
    builder_type: BuilderType,
) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;
    let ident_string = ident.to_string();

    let EventDataAttributes { entity } = parse_event_data_attributes(&input.attrs)?;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let builder_impl = match builder_type {
        BuilderType::Create => quote!(event_sauce::CreateEntityBuilder),
        BuilderType::Update => quote!(event_sauce::UpdateEntityBuilder),
    };

    Ok(quote!(
        impl #impl_generics event_sauce::EventData for #ident #ty_generics #where_clause {
            type Entity = #entity;

            const EVENT_TYPE: &'static str = #ident_string;
        }

        impl #builder_impl<#ident> for #entity {}
    ))
}

pub fn expand_derive_create_event_data(
    input: &DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => expand_derive_event_data_struct(input, named, BuilderType::Create),

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

pub fn expand_derive_update_event_data(
    input: &DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { named, .. }),
            ..
        }) => expand_derive_event_data_struct(input, named, BuilderType::Update),

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
