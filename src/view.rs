/// A read model built incrementally from an event stream.
///
/// `View` is analogous to [`crate::aggregate::Aggregate`] but is purely
/// reactive: it has no commands, no process step, and no error type. It is
/// intended to be used as a lightweight, in-memory or replayed projection of
/// events for query purposes.
///
/// # Example
/// ```rust
/// # use esrc::{Event, View};
/// #
/// #[derive(Event)]
/// enum CounterEvent {
///     Incremented { by: i64 },
///     Decremented { by: i64 },
/// }
///
/// #[derive(Default)]
/// struct CounterView {
///     value: i64,
/// }
///
/// impl View for CounterView {
///     type Event = CounterEvent;
///
///     fn apply(self, event: &Self::Event) -> Self {
///         match event {
///             CounterEvent::Incremented { by } => CounterView { value: self.value + by },
///             CounterEvent::Decremented { by } => CounterView { value: self.value - by },
///         }
///     }
/// }
/// ```
pub trait View: Default + Send {
    /// The event type that drives this view.
    ///
    /// All events replayed or received by this view must be of this type.
    type Event: crate::event::Event;

    /// Update the view state using a published event.
    ///
    /// This mirrors [`crate::aggregate::Aggregate::apply`] but is separate so
    /// that a type can independently implement both `Aggregate` and `View`
    /// without conflict.
    fn apply(self, event: &Self::Event) -> Self;
}
