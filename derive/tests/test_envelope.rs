use esrc::envelope::TryFromEnvelope as TryFromEnvelopeTrait;
use esrc::event::Event as EventTrait;
use esrc::version::DeserializeVersion as DeserializeVersionTrait;
use esrc_derive::TryFromEnvelope;

#[allow(dead_code)]
mod fixtures;

use fixtures::envelope::EmptyEnvelope;
use fixtures::event::{BarEvent, FooEvent, OwnedEvent};

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
fn try_from_envelope_generic() {
    #[derive(Debug, PartialEq, TryFromEnvelope)]
    enum TestGroup<T>
    where
        T: EventTrait + DeserializeVersionTrait,
    {
        Other(T),
        Owned(OwnedEvent),
    }

    let envelope = EmptyEnvelope::new(OwnedEvent::name());

    let expected = TestGroup::<FooEvent>::Owned(OwnedEvent {
        local_name: "LocalOwned".to_owned(),
    });
    let actual = TestGroup::<FooEvent>::try_from_envelope(&envelope).unwrap();

    assert_eq!(expected, actual);
}
