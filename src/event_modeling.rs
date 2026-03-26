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

/// A structured consumer identity derived from slice-oriented naming segments.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConsumerName {
    bounded_context: &'static str,
    domain: &'static str,
    feature: &'static str,
    consumer: &'static str,
}

impl ConsumerName {
    /// Create a new structured consumer identity.
    pub const fn new(
        bounded_context: &'static str,
        domain: &'static str,
        feature: &'static str,
        consumer: &'static str,
    ) -> Self {
        Self {
            bounded_context,
            domain,
            feature,
            consumer,
        }
    }

    /// The bounded context segment of this consumer identity.
    pub const fn bounded_context(&self) -> &'static str {
        self.bounded_context
    }

    /// The domain segment of this consumer identity.
    pub const fn domain(&self) -> &'static str {
        self.domain
    }

    /// The feature segment of this consumer identity.
    pub const fn feature(&self) -> &'static str {
        self.feature
    }

    /// The consumer segment of this consumer identity.
    pub const fn consumer(&self) -> &'static str {
        self.consumer
    }

    /// Returns the stable durable consumer name.
    pub fn durable_name(&self) -> String {
        format!(
            "{}_{}_{}_{}",
            self.bounded_context, self.domain, self.feature, self.consumer
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
    name: ConsumerName,
    role: ConsumerRole,
    execution_policy: ExecutionPolicy,
    projector: P,
}

impl<P> ConsumerSpec<P> {
    /// Create a new consumer specification with the given role defaults.
    pub fn new(name: ConsumerName, role: ConsumerRole, projector: P) -> Self {
        Self {
            name,
            role,
            execution_policy: role.default_execution_policy(),
            projector,
        }
    }

    /// Returns the structured name for this consumer.
    pub fn name(&self) -> &ConsumerName {
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
    pub fn new(name: ConsumerName, projector: P) -> Self {
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
    pub fn new(name: ConsumerName, projector: P) -> Self {
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
