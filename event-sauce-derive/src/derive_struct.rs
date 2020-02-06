use crate::ns::{diesel_table_name, StructInfo};
use proc_macro2::{Ident, Span, TokenStream};
use syn::{DataStruct, DeriveInput};

fn impl_event_data(parsed: &DeriveInput, struct_body: &DataStruct) -> TokenStream {
    let DeriveInput { ref ident, .. } = parsed;
    let StructInfo { target_model, .. } = StructInfo::new(&parsed, &struct_body);

    let ident_quoted = ident.to_string();

    quote! {
        use crate::event_store::EntityId;

        impl crate::event_store::EventData for #ident {
            fn entity_type() -> String {
                String::from(#target_model::entity_type())
            }

            fn event_type() -> String {
                String::from(#ident_quoted)
            }
        }
    }
}

pub fn derive_event_create(parsed: &DeriveInput, struct_body: &DataStruct) -> TokenStream {
    let DeriveInput { ref ident, .. } = parsed;

    let dummy_const = Ident::new(
        &format!("_IMPL_EVENT_STORE_STRUCT_FOR_{}", ident),
        Span::call_site(),
    );

    let event_data_impl = impl_event_data(parsed, struct_body);

    quote! {
        const #dummy_const: () = {
            #event_data_impl

            impl crate::event_store::FromCreatePayload<#ident> for crate::event_store::Event<#ident> {
                fn from_create_payload(data: #ident, session_id: Option<uuid::Uuid>) -> crate::event_store::Event<#ident> {
                    crate::event_store::Event {
                        data: Some(data),
                        session_id,
                        ..crate::event_store::Event::default()
                    }
                }
            }
        };
    }
}

pub fn derive_event_update(parsed: &DeriveInput, struct_body: &DataStruct) -> TokenStream {
    let DeriveInput { ref ident, .. } = parsed;

    let StructInfo { target_model, .. } = StructInfo::new(&parsed, &struct_body);

    let dummy_const = Ident::new(
        &format!("_IMPL_EVENT_STORE_STRUCT_FOR_{}", ident),
        Span::call_site(),
    );

    let event_data_impl = impl_event_data(parsed, struct_body);

    quote! {
        const #dummy_const: () = {
            #event_data_impl

            impl crate::event_store::FromUpdatePayload<#ident> for crate::event_store::Event<#ident> {
                type Entity = #target_model;

                fn from_update_payload(data: #ident, entity: &Self::Entity, session_id: Option<uuid::Uuid>) -> crate::event_store::Event<#ident> {
                    use crate::event_store::EntityId;

                    crate::event_store::Event {
                        data: Some(data),
                        entity_id: entity.entity_id(),
                        session_id,
                        ..crate::event_store::Event::default()
                    }
                }
            }
        };
    }
}

pub fn derive_event_delete(parsed: &DeriveInput, struct_body: &DataStruct) -> TokenStream {
    let DeriveInput { ref ident, .. } = parsed;

    let StructInfo { target_model, .. } = StructInfo::new(&parsed, &struct_body);

    let dummy_const = Ident::new(
        &format!("_IMPL_EVENT_STORE_STRUCT_FOR_{}", ident),
        Span::call_site(),
    );

    let event_data_impl = impl_event_data(parsed, struct_body);

    quote! {
        const #dummy_const: () = {
            #event_data_impl

            impl crate::event_store::FromDeletePayload<#ident> for crate::event_store::Event<#ident> {
                type Entity = #target_model;

                fn from_delete_payload(data: #ident, entity: &Self::Entity, session_id: Option<uuid::Uuid>) -> crate::event_store::Event<#ident> {
                    use crate::event_store::EntityId;

                    crate::event_store::Event {
                        data: Some(data),
                        entity_id: entity.entity_id(),
                        session_id,
                        ..crate::event_store::Event::default()
                    }
                }
            }
        };
    }
}

pub fn derive_entity(parsed: &DeriveInput, _struct_body: &DataStruct) -> TokenStream {
    let DeriveInput { ref ident, .. } = parsed;

    let entity_type =
        diesel_table_name(&parsed.attrs).expect("Missing attribute #[table_name = \"name\"]");

    let dummy_const = Ident::new(
        &format!("_IMPL_EVENT_STORE_STRUCT_FOR_{}", ident),
        Span::call_site(),
    );

    quote! {
        const #dummy_const: () = {
            impl crate::event_store::EntityId for #ident {
                fn entity_id(&self) -> Uuid {
                    self.id
                }

                fn entity_type() -> String {
                    String::from(#entity_type)
                }
            }
        };
    }
}
