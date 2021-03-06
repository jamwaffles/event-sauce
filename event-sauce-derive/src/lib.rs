extern crate proc_macro;

use proc_macro::TokenStream;

mod derives;

#[proc_macro_derive(Entity, attributes(event_sauce))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match derives::entity::expand_derive_entity(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(CreateEventData, attributes(event_sauce))]
pub fn derive_create_event_data(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match derives::event_data::expand_derive_create_event_data(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(UpdateEventData, attributes(event_sauce))]
pub fn derive_update_event_data(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match derives::event_data::expand_derive_update_event_data(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(DeleteEventData, attributes(event_sauce))]
pub fn derive_delete_event_data(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match derives::event_data::expand_derive_delete_event_data(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(EnumEventData, attributes(event_sauce))]
pub fn derive_action_event_data(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match derives::enum_event_data::expand_derive_enum_event_data(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}

#[proc_macro_derive(PurgeEventData, attributes(event_sauce))]
pub fn derive_purge_event_data(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match derives::event_data::expand_derive_purge_event_data(&input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
