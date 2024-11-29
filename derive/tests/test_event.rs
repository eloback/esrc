use std::marker::PhantomData;

use esrc::event::{Event, EventGroup};
use esrc_derive::{Event, EventGroup};

mod fixtures;

use fixtures::event::{BarEvent, FooEvent, LifetimeEvent};

#[test]
#[allow(unused)]
fn event() {
    #[derive(Event)]
    struct Test;

    assert_eq!("Test", Test::name());
}

#[test]
#[allow(unused)]
fn event_suffix() {
    #[derive(Event)]
    struct TestEvent;

    assert_eq!("Test", TestEvent::name());
}

#[test]
#[allow(unused)]
fn event_keep_suffix() {
    #[derive(Event)]
    #[esrc(event(keep_suffix))]
    struct TestEvent;

    assert_eq!("TestEvent", TestEvent::name());
}

#[test]
#[allow(unused)]
fn event_name() {
    #[derive(Event)]
    #[esrc(event(name = "Custom"))]
    struct TestEvent;

    assert_eq!("Custom", TestEvent::name());
}

#[test]
#[allow(unused)]
fn event_generics() {
    #[derive(Event)]
    struct TestEvent<'a>(PhantomData<&'a ()>);

    assert_eq!("Test", TestEvent::name());
}

#[test]
#[allow(unused)]
fn event_group() {
    #[derive(EventGroup)]
    enum TestGroup {
        Foo { foo: FooEvent },
        Bar(BarEvent),
    }

    let expected = ["Foo", "Bar"].into_iter();
    let actual = TestGroup::names();

    assert!(actual.eq(expected));
}

#[test]
#[allow(unused)]
fn event_group_ignore() {
    #[derive(EventGroup)]
    enum TestGroup {
        Foo {
            foo: FooEvent,
        },
        #[esrc(ignore)]
        Bad(i32),
        Bar(BarEvent),
    }

    let expected = ["Foo", "Bar"].into_iter();
    let actual = TestGroup::names();

    assert!(actual.eq(expected));
}

#[test]
#[allow(unused)]
fn event_group_lifetime() {
    #[derive(EventGroup)]
    enum TestGroup<'a, T: Event> {
        Foo(T),
        Lifetime(LifetimeEvent<'a>),
    }

    let expected = ["Foo", "Lifetime"].into_iter();
    let actual = TestGroup::<FooEvent>::names();

    assert!(actual.eq(expected))
}
