use std::marker::PhantomData;

use serde::{Deserialize, Serialize};

use esrc::version::{DeserializeVersion, SerializeVersion};
use esrc_derive::{DeserializeVersion, SerializeVersion};

mod fixtures;
use fixtures::version::unit_deserializer;

#[test]
#[allow(unused)]
fn deserialize_version() {
    #[derive(Debug, DeserializeVersion, PartialEq, Serialize)]
    struct TestEvent;

    impl<'de> Deserialize<'de> for TestEvent {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(TestEvent)
        }
    }

    impl SerializeVersion for TestEvent {
        fn version() -> usize {
            1
        }
    }

    let expected = TestEvent;
    let actual = TestEvent::deserialize_version(unit_deserializer(), 1);
    let invalid = TestEvent::deserialize_version(unit_deserializer(), 2);

    assert_eq!(expected, actual.unwrap());
    assert!(invalid.is_err());
}

#[test]
#[allow(unused)]
fn deserialize_version_lifetime() {
    #[derive(Debug, DeserializeVersion, PartialEq, Serialize)]
    struct TestEvent<'a>(PhantomData<&'a ()>);

    impl<'a, 'de: 'a> Deserialize<'de> for TestEvent<'a> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(TestEvent(PhantomData::default()))
        }
    }

    impl<'a> SerializeVersion for TestEvent<'a> {
        fn version() -> usize {
            2
        }
    }

    let expected = TestEvent(PhantomData::default());
    let actual = TestEvent::deserialize_version(unit_deserializer(), 2);

    assert_eq!(expected, actual.unwrap());
}

#[test]
#[allow(unused)]
fn deserialize_version_previous() {
    #[derive(Debug, DeserializeVersion, PartialEq, Serialize)]
    struct TestEvent1<'a>(usize, PhantomData<&'a ()>);

    #[derive(Debug, DeserializeVersion, PartialEq, Serialize)]
    #[esrc(serde(previous_version = "TestEvent1"))]
    struct TestEvent2<'a>(usize, PhantomData<&'a ()>);

    impl<'a, 'de: 'a> Deserialize<'de> for TestEvent1<'a> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            Ok(TestEvent1(11, PhantomData::default()))
        }
    }

    impl<'a, 'de: 'a> Deserialize<'de> for TestEvent2<'a> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de> {
            Ok(TestEvent2(22, PhantomData::default()))
        }
    }

    impl<'a> SerializeVersion for TestEvent1<'a> {
        fn version() -> usize {
            1
        }
    }

    impl<'a> SerializeVersion for TestEvent2<'a> {
        fn version() -> usize {
            2
        }
    }

    impl<'a> From<TestEvent1<'a>> for TestEvent2<'a> {
        fn from(value: TestEvent1) -> Self {
            TestEvent2(44, PhantomData::default())
        }
    }

    let expected = TestEvent2(44, PhantomData::default());
    let actual = TestEvent2::deserialize_version(unit_deserializer(), 1);

    assert_eq!(expected, actual.unwrap());
}

#[test]
#[allow(unused)]
fn serialize_version() {
    #[derive(Serialize, SerializeVersion)]
    #[esrc(serde(version = 2))]
    struct TestEvent;

    assert_eq!(2, TestEvent::version());
}

#[test]
#[allow(unused)]
fn serialize_version_default() {
    #[derive(Serialize, SerializeVersion)]
    struct TestEvent;

    assert_eq!(1, TestEvent::version());
}
