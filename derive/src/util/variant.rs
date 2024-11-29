use darling::FromVariant;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Type, Variant};

// Get all of the variants in an enum data type, applying the specified filter
// function based on the attribute argument paramter (for ignoring fields, etc).
pub fn try_collect<T, F>(input: &DeriveInput, filter: F) -> Result<Vec<Variant>, Error>
where
    T: FromVariant,
    F: Fn(&T) -> bool,
{
    if let Data::Enum(ref e) = input.data {
        Ok(e.variants
            .iter()
            .filter_map(|v| try_ignore(v, &filter).transpose())
            .collect::<Result<Vec<_>, _>>()?)
    } else {
        Err(Error::new(input.span(), "expected enum type"))
    }
}

// Get the inner type of a variant. The variant is expected to be a newtype with
// only one field, otherwise an error is returned.
pub fn try_into_inner_type(variant: Variant) -> Result<Type, Error> {
    if variant.fields.len() == 1 {
        Ok(variant.fields.into_iter().next().unwrap().ty)
    } else {
        Err(Error::new(variant.span(), "variant should have one field"))
    }
}

// Parse a variant's attributes and try to apply the specified filtering method.
// If the filter passes, a clone of the variant is returned, otherwise None.
fn try_ignore<T, F>(variant: &Variant, filter: &F) -> Result<Option<Variant>, Error>
where
    T: FromVariant,
    F: Fn(&T) -> bool,
{
    let args = T::from_variant(variant).map_err(Error::from)?;
    if filter(&args) {
        Ok(Some(variant.clone()))
    } else {
        Ok(None)
    }
}
