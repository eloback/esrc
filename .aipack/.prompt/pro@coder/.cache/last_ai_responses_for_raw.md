====
>   Info: Duration: 34s | ~$0.0593 | Prompt Tokens: 19,669 (cached: 17,152) | Completion Tokens: 3,247 
>  Model: gpt-5.4 (gpt-5.4-2026-03-05) | Adapter: OpenAIResp
>  Files: Context Files: 11 | Working Files: 0 | Knowledge Files: 1
>   Note: write_mode is true, so content below this line will **NOT** be included in next run/prompt.

<suggested_git_command>
git add -A . && git commit -m "+ event-modeling - add declaration types and builders"
</suggested_git_command>

<FILE_CHANGES>
<FILE_NEW file_path="src/event_modeling.rs">
use crate::project::Project;

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
    Concurrent { max_in_flight: usize },
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
            "{}.{}.{}.{}",
            self.bounded_context, self.domain, self.feature, self.consumer
        )
    }

    /// Returns the structured slice path without the consumer segment.
    pub fn slice_path(&self) -> String {
        format!(
            "{}.{}.{}",
            self.bounded_context, self.domain, self.feature
        )
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

impl<P> ConsumerSpec<P>
where
    P: Project,
{
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

impl<P> Automation<P>
where
    P: Project,
{
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

impl<P> ReadModel<P>
where
    P: Project,
{
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
</FILE_NEW>

<FILE_PATCH file_path="src/lib.rs">
@@
 /// Traits and helpers for the core Event type and event store implementations.
 pub mod event;
+/// Consumer declaration types for event modeling and vertical slices.
+pub mod event_modeling;
 /// Process events and perform side effects for Events outside of an Aggregate.
 pub mod project;
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-1-todo-steps.md">
@@
-## Step - implement the event_modeling module with declaration types and builders
-      status: not_started
-time-created: 2026-03-26 06:00:43
-
-Add the new `event_modeling` module and implement the declaration-facing types needed by vertical slices.
-
-- Create the module files and expose them from the crate.
-- Implement the declaration model planned in the previous step, including:
-  - consumer role representation
-  - execution policy representation
-  - structured consumer naming using bounded context, domain, and feature
-  - normalized consumer specification
-  - ergonomic `Automation` and `ReadModel` builders
-
-- Keep the implementation focused on declaration and normalization primitives, without wiring runtime execution yet.
-
-References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
-
 ## Step - integrate event_modeling declarations with NatsStore consumer startup
       status: not_started
 time-created: 2026-03-26 06:00:43
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-2-active-step.md">
@@
-## Step - define the event_modeling module surface and consumer declaration model
+## Step - implement the event_modeling module with declaration types and builders
       status: active
 time-created: 2026-03-26 06:00:43
-time-current: 2026-03-26 06:09:46
+time-current: 2026-03-26 06:12:05
 
-Create the new `event_modeling` module plan and define the initial public API surface for consumer declarations. This step should introduce the core declaration concepts discussed in the dev chat, with explicit support for consumer roles such as automation and read model, and stable slice-oriented naming based on bounded context, domain, and feature.
+Add the new `event_modeling` module and implement the declaration-facing types needed by vertical slices.
 
-- Define the planned types and responsibilities for:
-  - consumer identity and naming
-  - automation and read model declaration builders
-  - a normalized internal consumer specification
-  - execution policy defaults and overrides
+- Create the module files and expose them from the crate.
+- Implement the declaration model planned in the previous step, including:
+  - consumer role representation
+  - execution policy representation
+  - structured consumer naming using bounded context, domain, and feature
+  - normalized consumer specification
+  - ergonomic `Automation` and `ReadModel` builders
 
-- Capture the intended naming model so durable identities are derived from structured slice information instead of ad hoc strings.
+- Keep the implementation focused on declaration and normalization primitives, without wiring runtime execution yet.
 
-- Favor the direction described in `_workbench/consumers/dev-chat.md`, request `Expand the consumer declaration design with code examples`, especially step 5 of `My recommended practical path`, while intentionally skipping step 4 for now.
+References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
 
 ### Implementation Considerations
-- Public crate surface:
-  - add a new `event_modeling` module exposed from `src/lib.rs`
-  - keep the initial API declaration-focused, with no runtime coupling in this step
-
-- Planned declaration types:
-  - `ConsumerRole`, with explicit `Automation` and `ReadModel` variants
-  - `ExecutionPolicy`, with `Sequential` and bounded `Concurrent { max_in_flight }`
-  - `ConsumerName`, carrying structured `bounded_context`, `domain`, and `feature` segments, plus a stable consumer identifier
-  - `ConsumerSpec<P>`, as the normalized internal runtime-facing representation that holds identity, role, execution policy, and projector
-
-- Planned slice-facing builders:
-  - `Automation<P>` wrapping a normalized `ConsumerSpec<P>` and defaulting to concurrent execution
-  - `ReadModel<P>` wrapping a normalized `ConsumerSpec<P>` and defaulting to sequential execution
-  - both builders should normalize into `ConsumerSpec<P>` so runtime integration can later accept one shared shape
-
-- Naming model:
-  - durable consumer identities should be derived from structured slice information, not arbitrary strings
-  - the naming shape should be stable and slice-oriented, using `bounded_context.domain.feature.consumer`
-  - expose helpers that return the stable durable consumer name and a slice path representation, so startup and observability can share the same structured identity source
-
-- Runtime boundary for later steps:
-  - `NatsStore` should eventually accept `ConsumerSpec<P>` through a shared `run_consumer` entrypoint
-  - durable subscription creation, graceful shutdown, acking, and message processing remain infrastructure-owned and should not leak into declaration builders in this step
-
-- Intentional sequencing:
-  - this step defines the module surface and responsibilities only
-  - step 4 from `My recommended practical path` remains intentionally deferred for the initial implementation path, as requested by the todo step
+- Implemented the declaration-focused `src/event_modeling.rs` module and exposed it from `src/lib.rs`.
+- Added the initial declaration types:
+  - `ConsumerRole`
+  - `ExecutionPolicy`
+  - `ConsumerName`
+  - `ConsumerSpec<P>`
+- Added explicit slice-facing builders:
+  - `Automation<P>`
+  - `ReadModel<P>`
+- Kept the implementation focused on declaration and normalization primitives only, with no runtime wiring in this step.
+- Preserved the structured naming model through durable name and slice path helpers derived from `bounded_context`, `domain`, `feature`, and `consumer`.
</FILE_PATCH>

<FILE_PATCH file_path="_workbench/consumers/plan-3-done-steps.md">
@@
 ## Step - define the event_modeling module surface and consumer declaration model
       status: done
 time-created: 2026-03-26 06:00:43
    time-done: 2026-03-26 06:09:46
@@
 - Clarified the runtime boundary for later steps, preserving infrastructure ownership of subscription creation, shutdown, acking, and shared message processing.
 - Kept the sequencing aligned with the dev chat guidance by defining the surface first and deferring the later constructor-focused step as requested.
+
+## Step - implement the event_modeling module with declaration types and builders
+      status: done
+time-created: 2026-03-26 06:00:43
+   time-done: 2026-03-26 06:12:05
+
+Add the new `event_modeling` module and implement the declaration-facing types needed by vertical slices.
+
+- Create the module files and expose them from the crate.
+- Implement the declaration model planned in the previous step, including:
+  - consumer role representation
+  - execution policy representation
+  - structured consumer naming using bounded context, domain, and feature
+  - normalized consumer specification
+  - ergonomic `Automation` and `ReadModel` builders
+
+- Keep the implementation focused on declaration and normalization primitives, without wiring runtime execution yet.
+
+References: see the definition in `plan-2-active-step.md` or `plan-3-done-steps.md`, step `Step - define the event_modeling module surface and consumer declaration model`.
+
+### Summary
+- Added the new `event_modeling` module in `src/event_modeling.rs` and exposed it from `src/lib.rs`.
+- Implemented declaration-focused consumer primitives:
+  - `ConsumerRole`
+  - `ExecutionPolicy`
+  - `ConsumerName`
+  - `ConsumerSpec<P>`
+- Implemented explicit `Automation<P>` and `ReadModel<P>` builders that normalize into `ConsumerSpec<P>`.
+- Added structured naming helpers for both stable durable consumer names and slice path representation.
+- Kept the step scoped to declaration and normalization primitives, with no runtime startup integration yet.
</FILE_PATCH>
</FILE_CHANGES>




