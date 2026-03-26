/// The semantic role a consumer plays in the system.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConsumerRole {
    /// A consumer that reacts to events to trigger workflows, commands, or side effects.
    Automation,
    /// A consumer that materializes or updates read-oriented state from events.
    ReadModel,
}

impl ConsumerRole {
    /// The default execution policy for this role.
    pub fn default_execution_policy(self) -> ExecutionPolicy {
        match self {
            Self::Automation => ExecutionPolicy::Concurrent { max_in_flight: 16 },
            Self::ReadModel => ExecutionPolicy::Sequential,
        }
    }
}

/// The execution behavior a consumer should use when processing messages.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionPolicy {
    /// Process one message at a time, preserving consumer order.
    Sequential,
    /// Process multiple messages concurrently, up to the configured in-flight limit.
    Concurrent { 
        /// Maximum number of messages to process concurrently. Must be greater than zero.
        max_in_flight: usize
    },
}

/// A structured component identity derived from slice-oriented naming segments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentName {
    bounded_context: &'static str,
    domain: &'static str,
    feature: &'static str,
    component: &'static str,
}

impl ComponentName {
    /// Create a new structured component identity.
    pub const fn new(
        bounded_context: &'static str,
        domain: &'static str,
        feature: &'static str,
        component: &'static str,
    ) -> Self {
        Self {
            bounded_context,
            domain,
            feature,
            component,
        }
    }

    /// The bounded context segment of this component identity.
    pub const fn bounded_context(&self) -> &'static str {
        self.bounded_context
    }

    /// The domain segment of this component identity.
    pub const fn domain(&self) -> &'static str {
        self.domain
    }

    /// The feature segment of this component identity.
    pub const fn feature(&self) -> &'static str {
        self.feature
    }

    /// The component segment of this component identity.
    pub const fn component(&self) -> &'static str {
        self.component
    }

    /// Returns the stable durable consumer name.
    pub fn durable_name(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.bounded_context, self.domain, self.feature, self.component
        )
    }

    /// Returns the query service subject derived from this component name.
    ///
    /// The convention is `query.<bounded_context>.<domain>.<feature>.<component>`.
    pub fn query_subject(&self) -> String {
        format!(
            "query.{}.{}.{}.{}",
            self.bounded_context, self.domain, self.feature, self.component
        )
    }

    /// Returns the structured slice path without the consumer segment.
    pub fn slice_path(&self) -> String {
        format!("{}_{}_{}", self.bounded_context, self.domain, self.feature)
    }
}

/// A normalized consumer declaration that can later be executed by infrastructure.
#[derive(Clone, Debug)]
pub struct ConsumerSpec<P> {
    name: ComponentName,
    role: ConsumerRole,
    execution_policy: ExecutionPolicy,
    projector: P,
}

impl<P> ConsumerSpec<P> {
    /// Create a new consumer specification with the given role defaults.
    pub fn new(name: ComponentName, role: ConsumerRole, projector: P) -> Self {
        Self {
            name,
            role,
            execution_policy: role.default_execution_policy(),
            projector,
        }
    }

    /// Returns the structured name for this consumer.
    pub fn name(&self) -> &ComponentName {
        &self.name
    }

    /// Returns the semantic role for this consumer.
    pub fn role(&self) -> ConsumerRole {
        self.role
    }

    /// Returns the configured execution policy for this consumer.
    pub fn execution_policy(&self) -> ExecutionPolicy {
        self.execution_policy
    }

    /// Returns a reference to the configured projector.
    pub fn projector(&self) -> &P {
        &self.projector
    }

    /// Returns a mutable reference to the configured projector.
    pub fn projector_mut(&mut self) -> &mut P {
        &mut self.projector
    }

    /// Consumes the specification and returns the configured projector.
    pub fn into_projector(self) -> P {
        self.projector
    }

    /// Override the execution policy for this consumer.
    pub fn with_execution_policy(mut self, execution_policy: ExecutionPolicy) -> Self {
        self.execution_policy = execution_policy;
        self
    }
}

/// A declaration builder for automation consumers.
#[derive(Clone, Debug)]
pub struct Automation<P> {
    spec: ConsumerSpec<P>,
}

impl<P> Automation<P> {
    /// Create a new automation declaration with automation defaults.
    pub fn new(name: ComponentName, projector: P) -> Self {
        Self {
            spec: ConsumerSpec::new(name, ConsumerRole::Automation, projector),
        }
    }

    /// Override the execution policy for this automation.
    pub fn with_execution_policy(mut self, execution_policy: ExecutionPolicy) -> Self {
        self.spec = self.spec.with_execution_policy(execution_policy);
        self
    }

    /// Set the maximum in-flight concurrency for this automation.
    pub fn max_concurrency(self, max_in_flight: usize) -> Self {
        self.with_execution_policy(ExecutionPolicy::Concurrent { max_in_flight })
    }

    /// Returns the normalized consumer specification.
    pub fn as_spec(&self) -> &ConsumerSpec<P> {
        &self.spec
    }

    /// Consumes this builder and returns the normalized consumer specification.
    pub fn into_spec(self) -> ConsumerSpec<P> {
        self.spec
    }
}

/// A declaration builder for read model consumers.
#[derive(Clone, Debug)]
pub struct ReadModel<P> {
    spec: ConsumerSpec<P>,
}

impl<P> ReadModel<P> {
    /// Create a new read model declaration with read model defaults.
    pub fn new(name: ComponentName, projector: P) -> Self {
        Self {
            spec: ConsumerSpec::new(name, ConsumerRole::ReadModel, projector),
        }
    }

    /// Override the execution policy for this read model.
    pub fn with_execution_policy(mut self, execution_policy: ExecutionPolicy) -> Self {
        self.spec = self.spec.with_execution_policy(execution_policy);
        self
    }

    /// Returns the normalized consumer specification.
    pub fn as_spec(&self) -> &ConsumerSpec<P> {
        &self.spec
    }

    /// Consumes this builder and returns the normalized consumer specification.
    pub fn into_spec(self) -> ConsumerSpec<P> {
        self.spec
    }
}

/// A declaration builder that composes a read model consumer (`ConsumerSpec`)
/// and its query handler (`QuerySpec`) as a single vertical slice.
///
/// This simplifies the common case where a read model has both an event
/// consumer (write side) and a query handler (read side), allowing both to
/// be declared and spawned together.
///
/// # Example
///
/// ```rust,ignore
/// use esrc::event_modeling::{ComponentName, ReadModelSlice};
/// use esrc::query::QueryTransport;
///
/// let slice = ReadModelSlice::new(
///     ComponentName::new("shop", "catalog", "products", "list"),
///     my_projector,
///     my_kv_store,
/// );
///
/// // Spawn both the consumer and query service:
/// nats_store.spawn_read_model_slice(slice);
/// ```
#[derive(Clone, Debug)]
pub struct ReadModelSlice<P, H> {
    consumer_spec: ConsumerSpec<P>,
    query_spec: crate::query::QuerySpec<H>,
}

impl<P, H> ReadModelSlice<P, H> {
    /// Create a new read model slice with the given projector and query handler.
    ///
    /// The `ComponentName` is shared between the consumer and query specs.
    /// The consumer is created with `ConsumerRole::ReadModel` defaults
    /// (sequential execution). The query transport defaults to
    /// `NatsRequestReply`.
    pub fn new(name: ComponentName, projector: P, handler: H) -> Self {
        let consumer_spec = ConsumerSpec::new(name.clone(), ConsumerRole::ReadModel, projector);
        let query_spec = crate::query::QuerySpec::new(
            name,
            crate::query::QueryTransport::NatsRequestReply,
            handler,
        );

        Self {
            consumer_spec,
            query_spec,
        }
    }

    /// Override the execution policy for the consumer.
    pub fn with_execution_policy(mut self, execution_policy: ExecutionPolicy) -> Self {
        self.consumer_spec = self.consumer_spec.with_execution_policy(execution_policy);
        self
    }

    /// Override the transport for the query service.
    pub fn with_query_transport(mut self, transport: crate::query::QueryTransport) -> Self {
        self.query_spec = self.query_spec.with_transport(transport);
        self
    }

    /// Returns a reference to the consumer specification.
    pub fn consumer_spec(&self) -> &ConsumerSpec<P> {
        &self.consumer_spec
    }

    /// Returns a reference to the query specification.
    pub fn query_spec(&self) -> &crate::query::QuerySpec<H> {
        &self.query_spec
    }

    /// Consumes the slice and returns both specifications.
    pub fn into_specs(self) -> (ConsumerSpec<P>, crate::query::QuerySpec<H>) {
        (self.consumer_spec, self.query_spec)
    }

    /// Returns a reference to the shared component name.
    pub fn name(&self) -> &ComponentName {
        self.consumer_spec.name()
    }

    /// Returns a reference to the projector.
    pub fn projector(&self) -> &P {
        self.consumer_spec.projector()
    }

    /// Returns a mutable reference to the projector.
    pub fn projector_mut(&mut self) -> &mut P {
        self.consumer_spec.projector_mut()
    }

    /// Returns a reference to the query handler.
    pub fn handler(&self) -> &H {
        self.query_spec.handler()
    }

    /// Returns a mutable reference to the query handler.
    pub fn handler_mut(&mut self) -> &mut H {
        self.query_spec.handler_mut()
    }
}
