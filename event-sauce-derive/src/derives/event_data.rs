use proc_macro2::Span;
use quote::quote;
use syn::{Attribute, Data, DataStruct, DeriveInput, Fields, FieldsNamed, Meta, NestedMeta, Path};

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
    Delete,
    Purge,
    Action,
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
        BuilderType::Action => (
            quote!(event_sauce::ActionEntityBuilder),
            quote!(event_sauce::ActionEventBuilder),
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

fn expand_derive_event_data_enum(
    input: &DeriveInput,
    builder_type: BuilderType,
) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;

    let EventDataAttributes { entity } = parse_event_data_attributes(&input.attrs)?;

    if matches!(builder_type, BuilderType::Action) {
        let builder_impl = quote!(event_sauce::ActionEventBuilder);
        Ok(quote!(
            impl #builder_impl<#ident> for #entity {}
        ))
    } else {
        Err(syn::Error::new_spanned(
            input,
            "enums shall use action-builder only",
        ))
    }
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

pub fn expand_derive_action_event_data(
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
        }) => expand_derive_event_data_struct(input, BuilderType::Action),

        // TODO: this was added by me
        Data::Enum(_) => expand_derive_event_data_enum(input, BuilderType::Action),

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
