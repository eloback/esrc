# esrc
Primitives for implementing event sourcing and CQRS systems.

*Event sourcing* tracks state by storing changes to data as a stream of discrete
events. Rather than holding the most recent state in a database, event sourcing
holds the full history of changes to a domain object. This history can then be
"replayed" to construct the most up-to-date version of the domain object. This
pattern is useful for implementing *Command-Query Responsibility Segregation*
(CQRS), where the side of the app dealing with writes is separate from the side
that reads it. In this case, writes are commited to an event stream and used to
build up "projections"; separate read models (such as in a normal database) that
can be queried independently.

## Features

* Traits for representing events and aggregates
* Event versioning/upcasting
* Projections to build read models from one or more event streams
* Async interface for interacting with a generic event store
* Built-in event store implementation using NATS Jetstream
* Derive macros to simplify boilerplate

Some of these capabilities are gated behind crate feature flags:

| Feature  | Enables                                                   |
| -------- | --------------------------------------------------------- |
| `derive` | Procedural derive macro support (*default*)               |
| `nats`   | The NATS Jetstream event store implementation (*default*) |

## Getting Started

1. Define a domain model type.
2. Define an [`Event`] `enum`, whose variants represent possible state changes.
3. Define a Command `enum`, whose variants represent actions that result in the
   events defined in step (2).
4. Implement the [`Aggregate`] trait on the domain model. This trait defines how
   Commands should be processed, and how Events should be applied.
5. Create an event store. Currently, only a NATS implementation is available.
6. Read/write Events using the event store traits:
   * [`event::Publish`]: Write Events to the store. An extension trait also
     combines applying a command to an Aggregate and writing the created event.
   * [`event::Replay`]: Retrieve a stream of Event history for a specific
     Aggregate, or for all Aggregates across a set of events. Extension traits
	 also allow an Aggregate instance to be constructed as the stream is read,
	 and allow these past events to be Projected as described in step (7).
   * [`event::Subscribe`]: Listen to a stream of new events as they are written
     to the event store. An extension trait also allows these new events to be
	 Projected as they come in, as described in step (7).
   * [`event::Truncate`]: Remove old Events. If an event stream becomes too
     large, old events can be deleted. The Aggregate state after applying these
	 events should be persisted first (snapshotting), so that remaining events
	 can still be replayed correctly.
7. Construct read models from these events. The [`project::Project`] trait
   allows a callback to be defined that can process events of a specific type.

## More Examples

* `examples/cafe` provides an simplified implementation of the Edument CQRS
  tutorial at https://cqrs.nu/tutorial/Design. Involves writing events to a
  store and projecting them into a read model.
* `examples/zero_copy` shows how serde zero-copy deserialization can be used
  when encoding and decoding event data. Involves defining Event and Project
  types with lifetime annotations.
