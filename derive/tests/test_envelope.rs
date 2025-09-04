use esrc::envelope::TryFromEnvelope;
use esrc::event::Event;
use esrc::version::DeserializeVersion;
use esrc_derive::TryFromEnvelope;

mod fixtures;

use fixtures::envelope::EmptyEnvelope;
use fixtures::event::{BarEvent, FooEvent, LifetimeEvent};

#[test]
#[allow(unused)]
fn try_from_envelope() {
    #[derive(Debug, PartialEq, TryFromEnvelope)]
    enum TestGroup {
        Foo { foo: FooEvent },
        Bar(BarEvent),
    }

    let envelope = EmptyEnvelope::new(FooEvent::name());

    let expected = TestGroup::Foo { foo: FooEvent };
    let actual = TestGroup::try_from_envelope(&envelope).unwrap();

    assert_eq!(expected, actual);
}

#[test]
#[allow(unused)]
fn try_from_envelope_ignore() {
    #[derive(Debug, PartialEq, TryFromEnvelope)]
    enum TestGroup {
        Foo { foo: FooEvent },
        Bar(BarEvent),
    }

    let envelope = EmptyEnvelope::new(BarEvent::name());

    let expected = TestGroup::Bar(BarEvent);
    let actual = TestGroup::try_from_envelope(&envelope).unwrap();

    assert_eq!(expected, actual);
}

#[test]
#[allow(unused)]
fn try_from_envelope_lifetime() {
    #[derive(Debug, PartialEq, TryFromEnvelope)]
    enum TestGroup<'de, T>
    where
        T: Event + DeserializeVersion,
    {
        Other(T),
        Lifetime(LifetimeEvent<'de>),
    }

    let envelope = EmptyEnvelope::new(LifetimeEvent::name());

    let expected = TestGroup::<FooEvent>::Lifetime(LifetimeEvent {
        local_name: "LocalLifetime",
    });
    let actual = TestGroup::<FooEvent>::try_from_envelope(&envelope).unwrap();

    assert_eq!(expected, actual);
}
