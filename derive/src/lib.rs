use darling::FromDeriveInput;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

mod envelope;
mod event;
mod util;
mod version;

#[derive(FromDeriveInput)]
#[darling(attributes(esrc))]
struct EsrcAttributes {
    #[darling(default)]
    pub event: event::EventMeta,
    #[darling(default)]
    pub serde: version::SerdeMeta,
}

macro_rules! impl_derive {
    ($(#[$doc:meta])* $target:ident, $method:path $(,)?) => {
        #[proc_macro_derive($target, attributes(esrc))]
        #[allow(non_snake_case)]
        $(#[$doc])*
        pub fn $target(input: TokenStream) -> TokenStream {
            let input = parse_macro_input!(input as DeriveInput);
            let result = $method(input);

            result.unwrap_or_else(|e| e.into_compile_error()).into()
        }
    };
}

impl_derive!(
    /// Derive a DeserializeVersion implementation, with optional versioning.
    ///
    /// Deserialize this type according the the Deserialize implementation,
    /// if the version number matches what is defined in SerializeVersion.
    ///
    /// An `esrc(serde(previous_version))` type can also be specfied; in this
    /// case, that type's DeserializeVersion is used when the version does not
    /// match. A successful result is then upcasted with the
    /// [`From`](`std::convert::From`) implementation defined for the types.
    ///
    /// # Example
    /// ```rust
    /// # use esrc_derive::{DeserializeVersion, SerializeVersion};
    /// use serde::{Deserialize, Serialize};
    ///
    /// #[derive(Deserialize, DeserializeVersion, Serialize, SerializeVersion)]
    /// #[esrc(serde(version = 1))]
    /// struct OldEvent;
    ///
    /// #[derive(Deserialize, DeserializeVersion, Serialize, SerializeVersion)]
    /// #[esrc(serde(version = 2, previous_version = "OldEvent"))]
    /// struct NewEvent;
    ///
    /// impl From<OldEvent> for NewEvent {
    ///     fn from(value: OldEvent) -> Self {
    ///         NewEvent
    ///     }
    /// }
    /// ```
    DeserializeVersion,
    version::derive_deserialize_version,
);

impl_derive!(
    /// Derive an Event implementation, with optional custom naming.
    ///
    /// This defines the Event's `name` as the name of the derived type, minus
    /// the "Event" suffix (if present), unless attributes are specified:
    /// * `esrc(event(name))` will override the name to the value specified.
    ///   This is equivalent to implementing the trait manually.
    /// * `esrc(event(keep_suffix))` will not remove an "Event" suffix that is
    ///   present on the type name.
    ///
    /// # Example
    /// ```rust
    /// # use esrc::Event;
    /// # use esrc_derive::Event;
    ///
    /// #[derive(Event)]
    /// #[esrc(event(name = "BazEvent"))]
    /// struct FooEvent;
    ///
    /// #[derive(Event)]
    /// struct BarEvent;
    ///
    /// print!("{}", FooEvent::name()); // "BazEvent"
    /// print!("{}", BarEvent::name()); // "Bar"
    /// ```
    Event, event::derive_event
);

impl_derive!(
    /// Derive an EventGroup implementation for an enum type.
    ///
    /// This defines the EventGroup's `names` as a set of each variant's `name`
    /// (each variant is expected to implement the Event trait).
    ///
    /// An `esrc(ignore)` attribute may be specified on individual variants to
    /// prevent them from being included in this set (it also allows non-Event
    /// variants to be included in the type).
    EventGroup,
    event::derive_event_group,
);

impl_derive!(
    /// Derive a SerializeVersion implementation, defaulting to version 1.
    ///
    /// A custom version number can be specified as a positive integer with the
    /// `esrc(serde(version))` attribute.
    SerializeVersion,
    version::derive_serialize_version,
);

impl_derive!(
    /// Derive a TryFromEnvelope implementation for an enum type.
    ///
    /// This defines a method that uses the given Envelope's `name` to determine
    /// which variant it should be deserialized as (each variant is expected to
    /// implement the Event trait). If no match found, return an Invalid error.
    ///
    /// An `esrc(ignore)` attribute may be specified on individual variants to
    /// prevent them from being included in this set (it also allows non-Event
    /// variants to be included in the type).
    TryFromEnvelope,
    envelope::derive_try_from_envelope,
);
