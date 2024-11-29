use darling::{FromDeriveInput, FromMeta, FromVariant};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error};

use crate::util::variant;

#[derive(Default, FromMeta)]
pub struct EventMeta {
    pub name: Option<String>,

    #[darling(default)]
    pub keep_suffix: bool,
}

#[derive(FromVariant)]
#[darling(attributes(esrc))]
struct EventGroupArgs {
    #[darling(default)]
    pub ignore: bool,
}

pub fn derive_event(input: DeriveInput) -> Result<TokenStream, Error> {
    let args = super::EsrcAttributes::from_derive_input(&input).map_err(Error::from)?;

    let name = input.ident;
    let (impl_generics, ty_generics, clause) = input.generics.split_for_impl();

    // Retrieve the name of the event, either using the value provided in the
    // attribute macro, or the stringified type name.
    let name_string = args.event.name.unwrap_or(name.to_string());
    // Unless disabled in the attribute macro, try to remove the "Event" suffix
    // of the name. Note that this will also attempt to remove the suffix from
    // the name provided in the attribute, and would need to be disabled in the
    // attribute if manually naming a type `XyzEvent`.
    let event_name = if args.event.keep_suffix || name_string == "Event" {
        &name_string
    } else {
        name_string.strip_suffix("Event").unwrap_or(&name_string)
    };

    Ok(quote! {
        impl #impl_generics ::esrc::event::Event for #name #ty_generics #clause {
            fn name() -> &'static str {
                #event_name
            }
        }
    })
}

pub fn derive_event_group(input: DeriveInput) -> Result<TokenStream, Error> {
    // Get all of the (non-ignored) variants of the input enum. Retrieve the
    // type of each variant; the `name()` method from the Event trait will be
    // called, so the declaration of the inner field member does not matter.
    let variants = variant::try_collect(&input, |arg: &EventGroupArgs| !arg.ignore)?;
    let variant_tys = variants
        .into_iter()
        .map(variant::try_into_inner_type)
        .collect::<Result<Vec<_>, Error>>()?;

    let name = input.ident;
    let (impl_generics, ty_generics, clause) = input.generics.split_for_impl();

    // Generate the list of names by creating a constant list of the result of
    // each `name()` method, and convert it into an Iterator.
    Ok(quote! {
        impl #impl_generics ::esrc::event::EventGroup for #name #ty_generics #clause {
            fn names() -> impl Iterator<Item = &'static str> {
                [
                    #(<#variant_tys as ::esrc::event::Event>::name()),*
                ].into_iter()
            }
        }
    })
}
