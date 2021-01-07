use super::{parse_event_data_attributes, EventDataAttributes};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Variant};

fn impl_try_from(
    enum_ident: &Ident,
    variant_ident: &Ident,
) -> syn::Result<proc_macro2::TokenStream> {
    Ok(quote!(
        impl TryFrom<#enum_ident> for #variant_ident {
            type Error = ();

            fn try_from(value: #enum_ident) -> Result<Self, Self::Error> {
                match value {
                    #enum_ident::#variant_ident(e) => Ok(e),
                    _ => Err(()),
                }
            }
        }
    ))
}

fn expand_derive_event_data_enum(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;

    let EventDataAttributes { entity } = parse_event_data_attributes(&input.attrs)?;

    let variants = match input.data {
        Data::Enum(DataEnum { ref variants, .. }) => variants.iter(),
        _ => panic!("Input must be an enum"),
    };

    // TryFrom impls to convert enum payload into variant event data
    let conversions = variants
        .clone()
        .map(
            |Variant {
                 ident: variant_ident,
                 ..
             }| { impl_try_from(ident, variant_ident) },
        )
        .collect::<syn::Result<Vec<TokenStream>>>()?;

    let match_arms = variants
        .map(|Variant { ident: variant, .. }| quote!(#ident::#variant))
        .collect::<Vec<TokenStream>>();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    Ok(quote!(
        impl event_sauce::EventData for #ident {
            type Entity = #entity;

            type Builder = event_sauce::ActionEventBuilder<Self>;

            fn event_type(&self) -> &'static str {
                match self {
                    #(#match_arms(data) => data.event_type()),*
                }
            }
        }

        impl #impl_generics event_sauce::EnumEventData for #ident #ty_generics #where_clause {}

        impl event_sauce::ActionEntityBuilder<#ident> for #entity {}

        #(#conversions)*
    ))
}

pub fn expand_derive_enum_event_data(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    match &input.data {
        Data::Struct(_) => Err(syn::Error::new_spanned(input, "structs are not supported")),
        Data::Enum(_) => expand_derive_event_data_enum(input),
        Data::Union(_) => Err(syn::Error::new_spanned(input, "unions are not supported")),
    }
}
