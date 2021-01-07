use proc_macro2::Span;
use quote::quote;
use syn::MetaList;
use syn::{
    punctuated::Punctuated, token::Comma, Attribute, Data, DataStruct, DeriveInput, Field, Fields,
    FieldsNamed, Ident, Lit, Meta, MetaNameValue, NestedMeta,
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

/// Return the name of the field which is to become the entity ID field
fn find_entity_id_field(fields: &Punctuated<Field, Comma>) -> syn::Result<Ident> {
    // Find field with an attribute matching `#[event_sauce(id)]`
    let field = fields.iter().find(|field| {
        field
            .attrs
            .iter()
            .map(|attr| attr.parse_meta().expect("Invalid field attribute provided"))
            .any(|meta| match meta {
                Meta::List(MetaList { nested, .. }) if nested.len() == 1 => nested
                    .first()
                    .map(|nested_meta| matches!(nested_meta, NestedMeta::Meta(Meta::Path(path)) if path.is_ident("id")))
                    .unwrap_or(false),
                _ => false,
            })
    });

    if let Some(field_ident) = field.and_then(|f| f.ident.as_ref()) {
        Ok(field_ident.clone())
    } else {
        fail!(
            fields,
            "the #[event_sauce(id)] attribute is required on the ID field of the entity"
        )
    }
}

fn expand_derive_entity_struct(
    input: &DeriveInput,
    fields: &Punctuated<Field, Comma>,
) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;

    let EntityAttributes { entity_name } = parse_entity_attributes(&input.attrs)?;

    let entity_id_field = find_entity_id_field(&fields)?;

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote!(
        impl #impl_generics event_sauce::Entity for #ident #ty_generics #where_clause {
            const ENTITY_TYPE: &'static str = #entity_name;

            fn entity_id(&self) -> Uuid {
                self.#entity_id_field
            }
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
