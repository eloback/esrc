use quote::quote;
use syn::{GenericParam, Generics, Lifetime, LifetimeParam, WherePredicate};

pub fn add_supertype_bounds(mut generics: Generics, lifetime: &Lifetime) -> Generics {
    // Create a set of predicates defining the lifetime as outliving each
    // existing lifetime parameter, and add to the where clause.
    let predicates = generics
        .lifetimes()
        .map(|lt| as_predicate(lifetime, lt))
        .collect::<Vec<_>>();
    predicates
        .into_iter()
        .for_each(|p| generics.make_where_clause().predicates.push(p));

    // With the where clause updated, actually include the lifetime in the full
    // set of implementation generics.
    let lt_param = GenericParam::Lifetime(LifetimeParam::new(lifetime.clone()));
    generics.params.push(lt_param);

    generics
}

fn as_predicate(parent: &Lifetime, sub: &LifetimeParam) -> WherePredicate {
    let tokens = quote! { #parent: #sub };
    syn::parse::<WherePredicate>(tokens.into()).unwrap()
}
