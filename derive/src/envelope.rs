use darling::FromVariant;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::spanned::Spanned;
use syn::{DeriveInput, Error, Fields, Ident, Variant};

use crate::util::variant;

#[derive(FromVariant)]
#[darling(attributes(esrc))]
struct TryFromEnvelopeArgs {
    #[darling(default)]
    pub ignore: bool,
}

pub fn derive_try_from_envelope(input: DeriveInput) -> Result<TokenStream, Error> {
    let variants = variant::try_collect(&input, |arg: &TryFromEnvelopeArgs| !arg.ignore)?;

    let name = input.ident;
    let envelope = Ident::new("envelope", Span::call_site());

    let variant_branches = variants
        .into_iter()
        .map(|v| get_deserialize_branch(&name, &envelope, v))
        .collect::<Result<Vec<_>, Error>>()?;

    let (impl_generics, ty_generics, clause) = input.generics.split_for_impl();

    Ok(quote! {
        impl #impl_generics ::esrc::envelope::TryFromEnvelope for #name #ty_generics #clause {
            fn try_from_envelope(
                #envelope: &impl ::esrc::envelope::Envelope
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
    if variant.fields.len() != 1 {
        return Err(Error::new(
            variant.fields.span(),
            "TryFromEnvelope variant should have one field",
        ));
    }

    let field_ty = variant.fields.iter().next().unwrap().ty.clone();
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

    Ok(quote! {
        else if #envelope.name() == <#field_ty as ::esrc::event::Event>::name() {
            Ok(#field_ctor)
        }
    })
}
