use darling::FromVariant;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{DeriveInput, Error, Fields, Ident, Lifetime, Variant};

use crate::util::{lifetime, variant};

#[derive(FromVariant)]
#[darling(attributes(esrc))]
struct TryFromEnvelopeArgs {
    #[darling(default)]
    pub ignore: bool,
}

pub fn derive_try_from_envelope(input: DeriveInput) -> Result<TokenStream, Error> {
    // Get all of the (non-ignored) variants of the input enum.
    let variants = variant::try_collect(&input, |arg: &TryFromEnvelopeArgs| !arg.ignore)?;

    let name = input.ident;
    // Create a new envelope identifier, that is used by each branch of the
    // if-else conversion. This will be the parameter of the trait method.
    let envelope = Ident::new("envelope", Span::call_site());

    // Apply the envelope methods above to each variant, to create generic
    // branches for the if-else conversion chain.
    let variant_branches = variants
        .into_iter()
        .map(|v| get_deserialize_branch(&name, &envelope, v))
        .collect::<Result<Vec<_>, Error>>()?;

    // Create a new lifetime for the deserializer trait. This lifetime should
    // outlive all other lifetimes defined on the implementing type.
    let lt = Lifetime::new("'__esrc_de", Span::mixed_site());
    let generics = lifetime::add_supertype_bounds(input.generics.clone(), &lt);

    // Pull the type generics from the original generic data (before the new
    // lifetime was inserted), and the impl ones from the updated generics.
    let (_, ty_generics, _) = input.generics.split_for_impl();
    let (all_generics, _, clause) = generics.split_for_impl();

    // Build the try-from as an if-else chain, since the ::name() method of the
    // Event trait is not a constant expression (for using match). Start the
    // chain with an unreachable `if false`, try to deserialize each branch,
    // and then fail with an error if none succeed.
    Ok(quote! {
        impl #all_generics ::esrc::envelope::TryFromEnvelope<#lt> for #name #ty_generics #clause {
            fn try_from_envelope(
                #envelope: &#lt impl ::esrc::envelope::Envelope
            ) -> ::esrc::error::Result<Self> {
                if false {
                    unreachable!()
                }
                #(#variant_branches)*
                else {
                    Err(::esrc::error::Error::Invalid)
                }
            }
        }
    })
}

fn get_deserialize_branch(
    name: &Ident,
    envelope: &Ident,
    variant: Variant,
) -> Result<TokenStream, Error> {
    let variant_name = variant.ident.clone();
    // Each variant should have exactly one field, holding the corresponding
    // deserializable (event) type represented by this variant.
    if variant.fields.len() != 1 {
        return Err(Error::new(
            variant.fields.span(),
            "TryFromEnvelope variant should have one field",
        ));
    }

    let field_ty = variant.fields.iter().next().unwrap().ty.clone();
    // Support both tuple fields, and named fields for completeness, although
    // variants with named fields will always only have one name.
    let field_ctor = match variant.fields {
        Fields::Named(fields) => {
            let field_name = fields.named.into_iter().next().unwrap().ident;

            quote! {
                #name::#variant_name {
                    #field_name: #envelope.deserialize()?
                }
            }
        },
        Fields::Unnamed(_) => quote! {
            #name::#variant_name(#envelope.deserialize()?)
        },
        _ => return Err(Error::new(variant.span(), "bad enum field")),
    };

    // Create an else-if branch to be used in a larger statement. If the event
    // name from the envelope matches the trait-provided event name for this
    // variant, we can attempt to deserialize the envelope.
    Ok(quote! {
        else if #envelope.name() == <#field_ty as ::esrc::event::Event>::name() {
            Ok(#field_ctor)
        }
    })
}
