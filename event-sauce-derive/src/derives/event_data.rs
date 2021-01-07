use super::{parse_event_data_attributes, EventDataAttributes};
use quote::quote;
use syn::{Data, DataStruct, DeriveInput, Fields, FieldsNamed};

enum BuilderType {
    Create,
    Update,
    Delete,
    Purge,
}

fn expand_derive_event_data_struct(
    input: &DeriveInput,
    builder_type: BuilderType,
) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;
    let ident_string = ident.to_string();

    let EventDataAttributes { entity } = parse_event_data_attributes(&input.attrs)?;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    let (builder_impl, event_builder) = match builder_type {
        BuilderType::Create => (
            quote!(event_sauce::CreateEntityBuilder),
            quote!(event_sauce::CreateEventBuilder),
        ),
        BuilderType::Update => (
            quote!(event_sauce::UpdateEntityBuilder),
            quote!(event_sauce::UpdateEventBuilder),
        ),
        BuilderType::Delete => (
            quote!(event_sauce::DeleteEntityBuilder),
            quote!(event_sauce::DeleteEventBuilder),
        ),
        BuilderType::Purge => (
            quote!(event_sauce::PurgeEntityBuilder),
            quote!(event_sauce::PurgeEventBuilder),
        ),
    };

    Ok(quote!(
        impl #impl_generics event_sauce::EventData for #ident #ty_generics #where_clause {
            type Entity = #entity;

            type Builder = #event_builder <#ident>;

            fn event_type(&self) -> &'static str {
                #ident_string
            }
        }

        impl #builder_impl<#ident> for #entity {}
    ))
}

pub fn expand_derive_create_event_data(
    input: &DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { .. }),
            ..
        })
        | Data::Struct(DataStruct {
            fields: Fields::Unnamed(_),
            ..
        })
        | Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => expand_derive_event_data_struct(input, BuilderType::Create),

        Data::Enum(_) => Err(syn::Error::new_spanned(input, "enums are not supported")),

        Data::Union(_) => Err(syn::Error::new_spanned(input, "unions are not supported")),
    }
}

pub fn expand_derive_update_event_data(
    input: &DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { .. }),
            ..
        })
        | Data::Struct(DataStruct {
            fields: Fields::Unnamed(_),
            ..
        })
        | Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => expand_derive_event_data_struct(input, BuilderType::Update),

        Data::Enum(_) => Err(syn::Error::new_spanned(input, "enums are not supported")),

        Data::Union(_) => Err(syn::Error::new_spanned(input, "unions are not supported")),
    }
}

pub fn expand_derive_delete_event_data(
    input: &DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { .. }),
            ..
        })
        | Data::Struct(DataStruct {
            fields: Fields::Unnamed(_),
            ..
        })
        | Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => expand_derive_event_data_struct(input, BuilderType::Delete),

        Data::Enum(_) => Err(syn::Error::new_spanned(input, "enums are not supported")),

        Data::Union(_) => Err(syn::Error::new_spanned(input, "unions are not supported")),
    }
}

pub fn expand_derive_purge_event_data(
    input: &DeriveInput,
) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(FieldsNamed { .. }),
            ..
        })
        | Data::Struct(DataStruct {
            fields: Fields::Unnamed(_),
            ..
        })
        | Data::Struct(DataStruct {
            fields: Fields::Unit,
            ..
        }) => expand_derive_event_data_struct(input, BuilderType::Purge),

        Data::Enum(_) => Err(syn::Error::new_spanned(input, "enums are not supported")),

        Data::Union(_) => Err(syn::Error::new_spanned(input, "unions are not supported")),
    }
}
