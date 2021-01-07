pub mod entity;
pub mod enum_event_data;
pub mod event_data;

use proc_macro2::Span;
use syn::{Attribute, Meta, NestedMeta, Path};

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
            "Associated entity must be defined, e.g. #[event_sauce(User)]",
        )
    })?;

    Ok(EventDataAttributes { entity })
}
