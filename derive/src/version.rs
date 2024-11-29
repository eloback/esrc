use darling::{FromDeriveInput, FromMeta};
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{DeriveInput, Error, Ident, Lifetime};

use crate::util::lifetime;

#[derive(Default, FromMeta)]
pub struct SerdeMeta {
    pub version: Option<usize>,
    pub previous_version: Option<String>,
}

pub fn derive_deserialize_version(input: DeriveInput) -> Result<TokenStream, Error> {
    let args = super::EsrcAttributes::from_derive_input(&input).map_err(Error::from)?;
    let name = input.ident;

    // Create new identifiers for the version and deserializer, which are the
    // parameters for the deserialization method. These need to be referenced
    // in the optional previous version handling as well.
    let version = Ident::new("version", Span::call_site());
    let deserializer = Ident::new("deserializer", Span::call_site());

    // If a previous version is defined, attempt to deserialize it and convert
    // it to the current version, by calling is deserialize trait method.
    let previous = args
        .serde
        .previous_version
        .map(|p| get_from_previous(p, &version, &deserializer))
        .unwrap_or(quote! {
            Err(<D::Error as ::serde::de::Error>::custom("unknown version"))
        });

    // Create a new lifetime for the deserializer trait. This lifetime should
    // outlive all other lifetimes defined on the implementing type.
    let lt = Lifetime::new("'__esrc_de", Span::mixed_site());
    let generics = lifetime::add_supertype_bounds(input.generics.clone(), &lt);

    // Pull the type generics from the original generic data (before the new
    // lifetime was inserted), and the impl ones from the updated generics.
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let (all_generics, _, clause) = generics.split_for_impl();

    // Build the deserializer as an if-else chain, which first checks if the
    // received data has the expected version number to deserialize. If not,
    // pass along the attempt to the previous version.
    Ok(quote! {
        impl #all_generics ::esrc::version::DeserializeVersion<#lt> for #name #ty_generics #clause {
            fn deserialize_version<D>(#deserializer: D, #version: usize) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<#lt>,
            {
                if #version == <Self as ::esrc::version::SerializeVersion>::version() {
                    <Self as ::serde::Deserialize>::deserialize(#deserializer)
                } else {
                    #previous
                }
            }
        }
    })
}

pub fn derive_serialize_version(input: DeriveInput) -> Result<TokenStream, Error> {
    let args = super::EsrcAttributes::from_derive_input(&input).map_err(Error::from)?;
    // Pull the version from the attribute macro, or assume v1 as the default.
    let version = args.serde.version.unwrap_or(1);

    let name = input.ident;
    let (impl_generics, ty_generics, clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::esrc::version::SerializeVersion for #name #ty_generics #clause {
            fn version() -> usize {
                #version
            }
        }
    })
}

fn get_from_previous(previous_name: String, version: &Ident, deserializer: &Ident) -> TokenStream {
    let previous = Ident::new(previous_name.as_str(), Span::call_site());

    // Use the `From` implementation to convert from the old event version type.
    quote! {
        <#previous as ::esrc::version::DeserializeVersion>::deserialize_version(
            #deserializer,
            #version,
        )
        .map(Into::into)
    }
}
