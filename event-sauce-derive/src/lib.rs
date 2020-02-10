//! Generate event store code for events and entities
//!
//! # Examples
//!
//! See the [`backend::event_store`] module documentation for examples.
//!
//! [`backend::event_store`]: ../backend/index.html

#![recursion_limit = "128"]
#![deny(intra_doc_link_resolution_failure)]
#![deny(missing_docs)]

#[macro_use]
extern crate quote;
extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;

use proc_macro::TokenStream;
use syn::DeriveInput;

mod derive_struct;
mod ns;

const _PROC_MACRO_NAME: &'static str = "event_store";

/// Add functionality to a struct to allow it to be used as an entity creation event payload
#[proc_macro_derive(CreateEvent, attributes(event_store))]
pub fn derive_create_event(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();

    ns::expand_derive_event_create(&input).into()
}

/// Add functionality to a struct to allow it to be used as an entity update event payload
#[proc_macro_derive(UpdateEvent, attributes(event_store))]
pub fn derive_update_event(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();

    ns::expand_derive_event_update(&input).into()
}

/// Add functionality to a struct to allow it to be used as an entity deletion event
///
/// These events are most often left as empty structs. The entity ID is handled internally, so does
/// not need to be added to the payload.
#[proc_macro_derive(DeleteEvent, attributes(event_store))]
pub fn derive_delete_event(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();

    ns::expand_derive_event_delete(&input).into()
}

/// Add functionality to a struct to allow it to be created, updated and deleted by an event store
#[proc_macro_derive(Entity, attributes(event_store, table_name))]
pub fn derive_entity(input: TokenStream) -> TokenStream {
    let input: DeriveInput = syn::parse(input).unwrap();

    ns::expand_derive_entity(&input).into()
}
