# Dev Chat

Add a new `## Request: _user_ask_title_concise_` with the answer below (concise title). Use markdown sub-headings for sub sections. Keep this top instruction in this file.

## Original request

I'm trying to create a way to streamline consumer declaration on my vertical slices.
Most of the time i have two types of consumers:

- Automations: parallel/concurrent, run a Project with internal events and normally trigger a internal command, can do side effects or call external apis.
- Read Models: sequential, run a Project with internal events, normally update a persistent store, can call call external apis, normally don't do side effects.
  currently i have these functions in another branch, but i don't really like them.

give me your thoughts in how could i do this.

```rs
#[instrument(skip_all, level = "debug")]
       async fn durable_subscribe<G: EventGroup>(
           &self,
           unique_name: &str,
       ) -> error::Result<impl Stream<Item = error::Result<Self::Envelope>> + Send> {
           let (_, subjects) = {
               let mut names = G::names().collect::<Vec<_>>();
               names.sort();

               let subjects = names
                   .iter()
                   .map(|&n| NatsSubject::Event(n.into()).into_string(self.prefix))
                   .collect();
               (names.join("-"), subjects)
           };

           let consumer = self
               .durable_consumer(unique_name.to_string(), subjects)
               .await?;
           Ok(consumer
               .messages()
               .await?
               .map(|m| NatsEnvelope::try_from_message(self.prefix, m?)))
       }

       #[instrument(skip_all, level = "debug")]
       async fn start_automation<P>(
           &self,
           projector: P,
           feature_name: &str,
           max_concurrency: impl Into<Option<usize>> + Send,
       ) -> error::Result<()>
       where
           P: Project + 'static,
       {
           let stream = pin!(
               self.durable_subscribe::<P::EventGroup>(feature_name)
                   .await?,
           );

           let (exit, incoming) = Valved::new(stream);
           self.graceful_shutdown
               .exit_tx
               .send(exit)
               .await
               .expect("should be able to send graceful trigger");

           // Configure throughput (concurrent workers)
           incoming
               .for_each_concurrent(max_concurrency, |message| {
                   let projector = projector.clone();

                   async move {
                       if let Err(e) = NatsStore::process_message(&projector, message).await {
                           tracing::error!("Error processing message: {:?}", e);
                       }
                   }
               })
               .await;
           //// for read models the following code is used
           // while let Some(message) = incoming.next().await {
           //  let projector = projector.clone();
           //
           //  if let Err(e) = NatsStore::process_message(&projector, message).await {
           //      tracing::error!("Error processing message: {:?}", e);
           //  }

           Ok(())
       }
```

## Request: Streamline consumer declarations for automations and read models

### Summary

Your current direction makes sense, but I would avoid exposing low-level lifecycle details like `durable_subscribe` and `start_automation` as the main public model for vertical slices.

I think the cleanest shape is to make the slice declare intent, and let infrastructure derive the consumer mechanics from that intent.

The two categories you described are real and useful:

- Automations
- Read models

But I would not model them only as "one uses `for_each_concurrent`, one uses `while let Some(...)`".

I would model them as two consumer roles with different operational semantics.

### What feels off in the current API

#### `durable_subscribe`

This is reasonable as an internal primitive, but not a great abstraction boundary for feature code.

Why:

- The slice probably should not care that the implementation detail is a durable NATS consumer.
- The interesting thing at slice level is not "subscribe durably", it is "run this consumer with these guarantees".
- `unique_name: &str` is operationally necessary, but weak as a domain abstraction. It invites ad hoc naming.

So I would keep something like this internally, but not make it the declaration style that features build around.

#### `start_automation`

This is closer to the right level, but still mixes too many concerns:

- subscription creation
- shutdown wiring
- concurrency policy
- error handling strategy
- message processing
- feature identity

That usually leads to copy-paste variants later, like:

- start automation
- start read model
- start automation with retry
- start read model from mirror
- start automation with custom durable name
- start automation with ordered processing
- etc.

That is usually a sign the abstraction should move one level up.

### Recommended direction

### 1. Introduce a first-class "consumer declaration" concept

I would define one infrastructure-facing declaration per consumer, probably something conceptually like:

- feature name
- consumer name
- consumer kind
- projector
- delivery mode / ordering mode
- concurrency policy
- ack / retry / error policy
- source stream selection if needed

Not implementation now, just conceptually.

The important part is that a vertical slice declares:

- what it consumes
- what role it plays
- what execution guarantees it needs

Instead of declaring:

- how to create the stream
- how to iterate it
- how to wire shutdown
- how to branch automation vs read model manually

### 2. Treat "automation" and "read model" as roles, not just helpers

I think these should be explicit roles in your model.

For example, conceptually:

- `Automation`
  - concurrent by default
  - at-least-once processing
  - side effects allowed
  - command triggering expected
  - ordering usually not guaranteed globally
- `ReadModel`
  - sequential by default
  - deterministic handling preferred
  - persistence/update oriented
  - stronger expectation around ordering per consumer
  - side effects discouraged, or at least treated as exceptional

That gives you a better language for the framework and for the feature author.

### 3. Put concurrency and ordering behind policy, not separate loops

Instead of having "automation code path" and "read model code path", I would prefer a single runner that executes according to a policy.

Conceptually the runner gets:

- a message stream
- a processor
- an execution policy

And execution policy defines things like:

- `Sequential`
- `Concurrent { max_in_flight }`

Later you can extend with:

- `PartitionedByAggregate`
- `BoundedConcurrent`
- `SequentialWithBackoff`
- `ConcurrentWithRetryQueue`

This is useful because some automations may need sequential behavior, and some read models may later tolerate bounded concurrency. If you hard-code role -> loop shape, the model becomes rigid too early.

### 4. Separate declaration from runtime wiring

I would split the mental model into two layers:

#### Declaration layer

Owned by the vertical slice.

This should answer:

- which `Project`
- which role
- which stable consumer name
- which execution policy
- maybe whether it is optional / enabled

#### Runtime layer

Owned by infrastructure.

This should answer:

- create durable consumer
- attach to mirror or main stream
- process envelopes
- ack
- log
- retry / poison handling
- graceful shutdown
- task spawning

This keeps the slices simple and keeps NATS-specific mechanics out of their surface area.

### Naming thoughts

I would avoid names like `start_automation` as the main public abstraction if you also expect read models and maybe more consumer categories later.

Possible conceptual naming directions:

- `run_consumer`
- `start_consumer`
- `register_consumer`
- `spawn_consumer`
- `launch_consumer`

Then the declaration carries whether it is an automation or read model.

That is easier to scale than adding one start function per category.

### Consumer identity

`feature_name` and `unique_name` are the right idea, but I would strongly recommend a structured naming model.

You probably want a stable consumer identity derived from something like:

- bounded context / app
- slice or feature
- consumer role
- projector name or explicit consumer name

For example conceptually:

- `billing.invoice-read-model`
- `billing.send-payment-reminder`
- `identity.sync-crm`

This matters because durable consumers become operational assets, not just code details.

I would avoid arbitrary string arguments spread around call sites. Better to centralize naming in the declaration.

### Suggested shape for vertical slices

What I think would feel best is if each slice exposes a small list of consumers it owns.

Conceptually, not code:

- read models it runs
- automations it runs

Then app startup collects all declarations and passes them to a shared runtime.

That gives you:

- one place per slice to see all consumers
- less startup boilerplate
- easier consistency in naming and policies
- easier testing and introspection later

### How I would distinguish the two categories

### Automations

I would define them with these expectations:

- may run concurrently
- may trigger commands
- may call external APIs
- should be idempotent
- should tolerate redelivery
- should usually ack only after successful completion
- failures should be observable and maybe retried

Main design implication:

- treat them as workflow handlers, not just projections

Even if they implement `Project`, operationally they are different from read models.

You may eventually want a different trait name for them, even if they share the same envelope-to-handler pipeline underneath.

### Read models

I would define them with these expectations:

- usually sequential
- deterministic and replay-friendly
- update persistent views
- preferably no non-essential side effects
- should be restart-safe
- often should use a stable dedicated durable consumer
- often good candidates for mirrored streams / isolated read state

Main design implication:

- treat them as state builders

That difference is meaningful enough that I would preserve it in your declarations.

### Should both still use `Project`?

Short answer, yes for now, but maybe not forever.

Using `Project` for both is fine if:

- both consume the same typed event context
- both fit the same `project(context) -> Result<(), Error>` handling model

But conceptually, "automation" is often not really a projection. It is more like:

- react to event
- decide command
- invoke side effect
- maybe emit telemetry
- maybe schedule follow-up work

So I think this is okay as an implementation reuse strategy, but I would not force your external design language to pretend they are the same thing.

A good compromise is:

- keep `Project` internally as the handler contract
- wrap it in consumer declarations with role-specific semantics

### Strong recommendation, unify processing pipeline

Your `process_message` idea is good. I would lean into that.

You want one shared processing pipeline that does:

- decode envelope
- create `Context`
- call handler
- map errors
- ack
- log/tracing
- maybe metrics

Then your consumer runtime only varies in:

- stream source
- consumer identity
- execution policy
- error strategy

This reduces divergence between automation and read model runners.

### Error strategy matters more than it looks

This is the part I would design explicitly now, even if you do not implement it yet.

Because automations and read models often want different failure semantics.

For example:

- Read model failure may need to stop the consumer immediately, preserving order and consistency.
- Automation failure may be okay to retry, dead-letter, or park, depending on the side effect.

If this is not modeled, the runtime tends to accumulate hidden special cases.

So in the declaration, I would eventually want a policy like:

- stop on first error
- log and continue
- retry with backoff
- send to dead letter / park stream

Even if v1 only supports one policy, designing for it now will help.

### About graceful shutdown

I would definitely keep shutdown management out of slice-facing APIs.

Slices should not know about:

- valves
- triggers
- trackers
- internal task control

They should just declare consumers, and the runtime should own lifecycle.

That means your current shutdown wiring is good infrastructure code, but not a good declaration pattern for feature code.

### Suggested architectural model

### Option A, one trait for all declared consumers

A single declaration abstraction, with role + policy fields.

Pros:

- simple
- scalable
- one registry / startup path
- role-specific behavior still possible

This is the approach I would choose first.

### Option B, separate declaration types for automations and read models

Two declarations:

- automation declaration
- read model declaration

Pros:

- clearer ergonomics for slice authors
- better defaults
- stronger semantic separation

Cons:

- some duplication
- might later need shared internal normalization anyway

This is also a good option if you want the API to feel very obvious to application developers.

My bias:

- external API: separate declaration builders or constructors for ergonomics
- internal runtime: normalize both into one generic consumer spec

That tends to give the best of both worlds.

### Example of the design direction I would aim for

Not implementation, just the target feel:

- a slice declares a read model consumer with a stable name and default sequential execution
- a slice declares an automation consumer with a stable name and configured concurrency
- startup registers both with a shared runtime
- runtime owns durable naming, shutdown, acking, retries, tracing, and task spawning

So the slice says:

- "run this projector as a read model"
- "run this projector as an automation with max concurrency N"

Not:

- "subscribe durably, valve the stream, wire shutdown, choose the loop, clone the projector, process the message"

That is the key simplification I would chase.

### Practical recommendation for your next iteration

If I were refining this branch, I would do this conceptually:

- keep `durable_subscribe` private infrastructure
- keep a single shared `process_message`
- introduce consumer declarations
- give declarations a `role`
- give declarations an execution policy
- make startup consume declarations
- derive durable names from declaration identity, not ad hoc string arguments
- keep graceful shutdown fully inside runtime

### Final opinion

I think your intuition is correct, the current functions are useful building blocks, but not the final API you want for vertical slices.

The strongest design is:

- slices declare consumers
- declarations express role and guarantees
- runtime handles transport and lifecycle

That keeps your slice code intention-revealing, keeps NATS concerns localized, and gives you room to evolve retry, ordering, mirrors, partitioning, and observability later without changing every feature.

## Request: Expand the consumer declaration design with code examples

### Summary

Yes, I think the best next step is to make the idea more concrete with code shapes, while still keeping the transport and runtime details hidden from vertical slices.

The main goal is still the same:

- slices declare intent
- infrastructure owns lifecycle and NATS details
- automation vs read model remains explicit
- execution policy is configurable without duplicating the runner

Below I will explore a few increasingly concrete design options.

### Design target

What I would optimize for:

- a slice can declare consumers in one place
- startup can register all consumers consistently
- durable names are stable and not ad hoc
- `Project` can still be reused as the processing contract
- concurrency, ordering, retries, and shutdown remain infrastructure concerns

### 1. Start with a small declaration object

A good first step is a single normalized declaration type that infrastructure can run.

~~~rs
pub enum ConsumerRole {
    Automation,
    ReadModel,
}

pub enum ExecutionPolicy {
    Sequential,
    Concurrent { max_in_flight: usize },
}

pub enum ErrorPolicy {
    Stop,
    LogAndContinue,
}

pub struct ConsumerName {
    pub feature: &'static str,
    pub consumer: &'static str,
}

impl ConsumerName {
    pub fn durable_name(&self) -> String {
        format!("{}.{}", self.feature, self.consumer)
    }
}

pub struct ConsumerSpec<P> {
    pub name: ConsumerName,
    pub role: ConsumerRole,
    pub execution: ExecutionPolicy,
    pub error_policy: ErrorPolicy,
    pub projector: P,
}
~~~

This gives you a single internal runtime model that can be executed generically.

The important part is that the spec contains stable identity and policy, not transport-level behavior.

### Why this is a good baseline

It gives you:

- explicit semantics
- stable naming
- one runtime path
- room to extend later

It also avoids locking "automation = concurrent forever" or "read model = sequential forever" into hard-coded API branching.

You can still apply good defaults by role, but not make the role the only thing that controls execution.

### 2. Add ergonomic constructors for slice authors

The normalized type above is good for the runtime, but slice code should be even more intention-revealing.

I would likely add constructors like this:

~~~rs
impl<P> ConsumerSpec<P> {
    pub fn automation(
        feature: &'static str,
        consumer: &'static str,
        projector: P,
    ) -> Self {
        Self {
            name: ConsumerName { feature, consumer },
            role: ConsumerRole::Automation,
            execution: ExecutionPolicy::Concurrent { max_in_flight: 16 },
            error_policy: ErrorPolicy::Stop,
            projector,
        }
    }

    pub fn read_model(
        feature: &'static str,
        consumer: &'static str,
        projector: P,
    ) -> Self {
        Self {
            name: ConsumerName { feature, consumer },
            role: ConsumerRole::ReadModel,
            execution: ExecutionPolicy::Sequential,
            error_policy: ErrorPolicy::Stop,
            projector,
        }
    }

    pub fn with_execution(mut self, execution: ExecutionPolicy) -> Self {
        self.execution = execution;
        self
    }

    pub fn with_error_policy(mut self, error_policy: ErrorPolicy) -> Self {
        self.error_policy = error_policy;
        self
    }
}
~~~

This gives slices a simple declaration language:

- `automation(...)`
- `read_model(...)`
- optional policy overrides

That reads much better than wiring NATS subscriptions manually.

### 3. What a vertical slice could look like

Suppose a billing slice owns:

- one read model that updates invoice projections
- one automation that sends reminders

A slice-facing declaration module could look like this:

~~~rs
use crate::consumer::{ConsumerSpec, ExecutionPolicy};
use crate::billing::projectors::{InvoiceReadModel, SendPaymentReminder};

pub fn consumers() -> Vec<ConsumerSpec<Box<dyn RunnableProject>>> {
    vec![
        ConsumerSpec::read_model(
            "billing",
            "invoice-read-model",
            Box::new(InvoiceReadModel::new()),
        ),
        ConsumerSpec::automation(
            "billing",
            "send-payment-reminder",
            Box::new(SendPaymentReminder::new()),
        )
        .with_execution(ExecutionPolicy::Concurrent { max_in_flight: 32 }),
    ]
}
~~~

I would not necessarily force `Box<dyn RunnableProject>` immediately, but this demonstrates the target feel.

The important ergonomic property is that the slice declares all owned consumers in one place.

### 4. If you want stronger typing, keep generic declarations

If you want to preserve compile-time type information longer, you can keep declarations generic and let startup register them individually.

~~~rs
pub fn invoice_read_model() -> ConsumerSpec<InvoiceReadModel> {
    ConsumerSpec::read_model(
        "billing",
        "invoice-read-model",
        InvoiceReadModel::new(),
    )
}

pub fn send_payment_reminder() -> ConsumerSpec<SendPaymentReminder> {
    ConsumerSpec::automation(
        "billing",
        "send-payment-reminder",
        SendPaymentReminder::new(),
    )
    .with_execution(ExecutionPolicy::Concurrent { max_in_flight: 32 })
}
~~~

Then startup can do:

~~~rs
runtime.register(billing::consumers::invoice_read_model()).await?;
runtime.register(billing::consumers::send_payment_reminder()).await?;
~~~

This may be more practical if you do not want to introduce trait objects yet.

### 5. Shared runtime runner

I think the runtime should own one generic entrypoint.

Conceptually something like:

~~~rs
impl NatsStore {
    pub async fn run_consumer<P>(&self, spec: ConsumerSpec<P>) -> error::Result<()>
    where
        P: Project + Send + Sync + Clone + 'static,
    {
        let durable_name = spec.name.durable_name();
        let stream = self
            .durable_subscribe::<P::EventGroup>(&durable_name)
            .await?;

        self.run_stream(spec, stream).await
    }
}
~~~

And then execution shape is delegated to a policy-based runner:

~~~rs
impl NatsStore {
    async fn run_stream<P, S>(
        &self,
        spec: ConsumerSpec<P>,
        stream: S,
    ) -> error::Result<()>
    where
        P: Project + Send + Sync + Clone + 'static,
        S: futures::Stream<Item = error::Result<NatsEnvelope>> + Send,
    {
        match spec.execution {
            ExecutionPolicy::Sequential => {
                self.run_sequential(spec, stream).await
            }
            ExecutionPolicy::Concurrent { max_in_flight } => {
                self.run_concurrent(spec, stream, max_in_flight).await
            }
        }
    }
}
~~~

This is where the abstraction starts paying off.

### 6. Sequential and concurrent runners

The core idea is that both paths share the same message processor.

~~~rs
impl NatsStore {
    async fn run_sequential<P, S>(
        &self,
        spec: ConsumerSpec<P>,
        mut stream: S,
    ) -> error::Result<()>
    where
        P: Project + Send + Sync + Clone + 'static,
        S: futures::Stream<Item = error::Result<NatsEnvelope>> + Send + Unpin,
    {
        while let Some(message) = stream.next().await {
            self.handle_message(&spec, message).await?;
        }

        Ok(())
    }

    async fn run_concurrent<P, S>(
        &self,
        spec: ConsumerSpec<P>,
        stream: S,
        max_in_flight: usize,
    ) -> error::Result<()>
    where
        P: Project + Send + Sync + Clone + 'static,
        S: futures::Stream<Item = error::Result<NatsEnvelope>> + Send,
    {
        stream
            .for_each_concurrent(Some(max_in_flight), |message| {
                let spec = spec.clone();

                async move {
                    if let Err(err) = self.handle_message(&spec, message).await {
                        match spec.error_policy {
                            ErrorPolicy::Stop => {
                                tracing::error!(
                                    consumer = %spec.name.durable_name(),
                                    ?err,
                                    "consumer message handling failed"
                                );
                            }
                            ErrorPolicy::LogAndContinue => {
                                tracing::warn!(
                                    consumer = %spec.name.durable_name(),
                                    ?err,
                                    "consumer message handling failed"
                                );
                            }
                        }
                    }
                }
            })
            .await;

        Ok(())
    }
}
~~~

I would not copy this literally into production as-is, but this shows the structure I think is healthy.

### Important nuance on `ErrorPolicy::Stop`

With concurrent runners, "stop on first error" is more subtle than in the sequential case.

If you eventually support a strict stop semantic for concurrent mode, you will likely want:

- cancellation support
- draining behavior
- maybe a task group abstraction
- clear ack behavior for in-flight messages

So for v1, you may decide:

- sequential + stop is fully supported
- concurrent mode initially logs failures and leaves retry/redelivery to infrastructure semantics

That would be a perfectly reasonable staged design.

### 7. Shared message processing pipeline

This is the part I would definitely centralize.

~~~rs
impl NatsStore {
    async fn handle_message<P>(
        &self,
        spec: &ConsumerSpec<P>,
        message: error::Result<NatsEnvelope>,
    ) -> error::Result<()>
    where
        P: Project + Send + Sync,
    {
        let envelope = message?;
        let context = crate::project::Context::try_with_envelope(&envelope)?;

        let mut projector = spec.projector.clone();
        projector
            .project(context)
            .await
            .map_err(|e| crate::error::Error::External(e.into()))?;

        envelope.ack().await;
        Ok(())
    }
}
~~~

This function is where you can consistently handle:

- decoding
- typed `Context`
- projector execution
- acking
- metrics
- tracing
- error conversion

This should be shared no matter whether the consumer is an automation or a read model.

### 8. Role-specific defaults without role-specific runtime duplication

I would keep role in the model, but mostly use it for:

- defaults
- observability
- future policy choices

For example:

~~~rs
impl ConsumerRole {
    pub fn default_execution(&self) -> ExecutionPolicy {
        match self {
            ConsumerRole::Automation => ExecutionPolicy::Concurrent { max_in_flight: 16 },
            ConsumerRole::ReadModel => ExecutionPolicy::Sequential,
        }
    }
}
~~~

And maybe also for metrics:

~~~rs
tracing::info!(
    role = ?spec.role,
    consumer = %spec.name.durable_name(),
    "starting consumer"
);
~~~

This preserves the semantic distinction, without forcing all runtime branching to be duplicated by role.

### 9. If you want stronger semantics, split declaration API but normalize internally

This may actually be the nicest compromise.

Slice-facing API:

~~~rs
pub struct Automation<P> {
    inner: ConsumerSpec<P>,
}

pub struct ReadModel<P> {
    inner: ConsumerSpec<P>,
}
~~~

With constructors:

~~~rs
impl<P> Automation<P> {
    pub fn new(
        feature: &'static str,
        consumer: &'static str,
        projector: P,
    ) -> Self {
        Self {
            inner: ConsumerSpec::automation(feature, consumer, projector),
        }
    }

    pub fn max_concurrency(mut self, value: usize) -> Self {
        self.inner.execution = ExecutionPolicy::Concurrent {
            max_in_flight: value,
        };
        self
    }

    pub fn into_spec(self) -> ConsumerSpec<P> {
        self.inner
    }
}

impl<P> ReadModel<P> {
    pub fn new(
        feature: &'static str,
        consumer: &'static str,
        projector: P,
    ) -> Self {
        Self {
            inner: ConsumerSpec::read_model(feature, consumer, projector),
        }
    }

    pub fn into_spec(self) -> ConsumerSpec<P> {
        self.inner
    }
}
~~~

This gives a very expressive slice API:

~~~rs
pub fn send_payment_reminder() -> Automation<SendPaymentReminder> {
    Automation::new(
        "billing",
        "send-payment-reminder",
        SendPaymentReminder::new(),
    )
    .max_concurrency(32)
}

pub fn invoice_read_model() -> ReadModel<InvoiceReadModel> {
    ReadModel::new(
        "billing",
        "invoice-read-model",
        InvoiceReadModel::new(),
    )
}
~~~

Internally, both become `ConsumerSpec<P>` and use the same runtime path.

I think this is one of the strongest designs if your priority is feature ergonomics.

### 10. Example using your existing `Project` trait

Using your current trait shape, a read model might still look normal:

~~~rs
#[derive(Clone)]
pub struct InvoiceReadModel {
    repo: InvoiceViewRepository,
}

impl<'de> Project<'de> for InvoiceReadModel {
    type EventGroup = BillingEvent;
    type Error = InvoiceReadModelError;

    async fn project<E: Envelope>(
        &mut self,
        context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error> {
        match &*context {
            BillingEvent::InvoiceCreated(event) => {
                self.repo.create(context.id(), event).await?;
            }
            BillingEvent::InvoicePaid(event) => {
                self.repo.mark_paid(context.id(), event).await?;
            }
            BillingEvent::InvoiceCancelled(event) => {
                self.repo.cancel(context.id(), event).await?;
            }
        }

        Ok(())
    }
}
~~~

And an automation could still use the same trait even though semantically it is not really a projection:

~~~rs
#[derive(Clone)]
pub struct SendPaymentReminder {
    command_bus: BillingCommandBus,
    gateway: ReminderGateway,
}

impl<'de> Project<'de> for SendPaymentReminder {
    type EventGroup = BillingEvent;
    type Error = SendPaymentReminderError;

    async fn project<E: Envelope>(
        &mut self,
        context: Context<'de, E, Self::EventGroup>,
    ) -> Result<(), Self::Error> {
        match &*context {
            BillingEvent::InvoiceOverdue(event) => {
                self.gateway.send_reminder(event.customer_id).await?;
                self.command_bus
                    .dispatch(MarkReminderSent {
                        invoice_id: context.id(),
                    })
                    .await?;
            }
            _ => {}
        }

        Ok(())
    }
}
~~~

That is why I think reusing `Project` is fine for now, even if the role semantics differ.

### 11. Start-up composition

At application startup, I would try to centralize registration.

For example:

~~~rs
pub async fn start_consumers(store: &NatsStore) -> error::Result<()> {
    store
        .run_consumer(
            ReadModel::new(
                "billing",
                "invoice-read-model",
                InvoiceReadModel::new(),
            )
            .into_spec(),
        )
        .await?;

    store
        .run_consumer(
            Automation::new(
                "billing",
                "send-payment-reminder",
                SendPaymentReminder::new(),
            )
            .max_concurrency(32)
            .into_spec(),
        )
        .await?;

    Ok(())
}
~~~

Or if you want them spawned in the background:

~~~rs
pub async fn spawn_consumers(store: NatsStore) -> error::Result<()> {
    let billing_read_model = store.clone();
    store.get_task_tracker().spawn(async move {
        if let Err(err) = billing_read_model
            .run_consumer(
                ReadModel::new(
                    "billing",
                    "invoice-read-model",
                    InvoiceReadModel::new(),
                )
                .into_spec(),
            )
            .await
        {
            tracing::error!(?err, "billing invoice read model stopped");
        }
    });

    let billing_automation = store.clone();
    store.get_task_tracker().spawn(async move {
        if let Err(err) = billing_automation
            .run_consumer(
                Automation::new(
                    "billing",
                    "send-payment-reminder",
                    SendPaymentReminder::new(),
                )
                .max_concurrency(32)
                .into_spec(),
            )
            .await
        {
            tracing::error!(?err, "billing reminder automation stopped");
        }
    });

    Ok(())
}
~~~

This keeps lifecycle and task management in infrastructure and startup code, not inside the slices themselves.

### 12. Stable consumer naming strategy

I strongly recommend standardizing naming.

A simple approach:

~~~rs
pub struct ConsumerName {
    bounded_context: &'static str,
    feature: &'static str,
    consumer: &'static str,
}

impl ConsumerName {
    pub fn durable_name(&self) -> String {
        format!(
            "{}.{}.{}",
            self.bounded_context,
            self.feature,
            self.consumer
        )
    }
}
~~~

Then examples become:

- `billing.invoices.read-model`
- `billing.invoices.send-payment-reminder`
- `identity.users.sync-crm`

This gives you stable operational names and avoids random strings at call sites.

### 13. Future extension points that fit naturally

This model scales nicely to future needs.

#### Source selection

If you want mirror support later:

~~~rs
pub enum ConsumerSource {
    MainStream,
    Mirror(&'static str),
}
~~~

#### Retry policy

~~~rs
pub enum RetryPolicy {
    None,
    Immediate { max_retries: usize },
    Backoff {
        max_retries: usize,
        initial_millis: u64,
        max_millis: u64,
    },
}
~~~

#### Ordering policy

~~~rs
pub enum OrderingPolicy {
    GlobalSequential,
    ConcurrentUnordered,
    PartitionedByAggregate { max_in_flight: usize },
}
~~~

That last one is especially interesting for automations.

A lot of automations do not require global ordering, but do benefit from preserving ordering per aggregate ID. If you eventually support that, your policy-driven design will make room for it.

### 14. A more advanced model, `ConsumerDefinition`

If you want to fully decouple declaration from startup registration, you could define a trait for declarative consumer modules.

~~~rs
pub trait ConsumerDefinition {
    type Projector: Clone;

    fn feature(&self) -> &'static str;
    fn consumer(&self) -> &'static str;
    fn role(&self) -> ConsumerRole;
    fn execution(&self) -> ExecutionPolicy;
    fn projector(&self) -> Self::Projector;
}
~~~

Then a slice can define concrete consumer definitions:

~~~rs
pub struct InvoiceReadModelConsumer;

impl ConsumerDefinition for InvoiceReadModelConsumer {
    type Projector = InvoiceReadModel;

    fn feature(&self) -> &'static str {
        "billing"
    }

    fn consumer(&self) -> &'static str {
        "invoice-read-model"
    }

    fn role(&self) -> ConsumerRole {
        ConsumerRole::ReadModel
    }

    fn execution(&self) -> ExecutionPolicy {
        ExecutionPolicy::Sequential
    }

    fn projector(&self) -> Self::Projector {
        InvoiceReadModel::new()
    }
}
~~~

Personally, I would only do this if you have enough slices that explicit declaration objects start feeling repetitive.

For many codebases, simple constructors are enough.

### 15. My recommended practical path

If I were implementing this incrementally, I would do it in this order:

#### Step 1

Create an internal `ConsumerSpec<P>` with:

- consumer name
- role
- execution policy
- projector

#### Step 2

Implement one `run_consumer(spec)` runtime entrypoint.

#### Step 3

Centralize message handling in one `handle_message`.

#### Step 4

Add ergonomic constructors:

- `ConsumerSpec::automation(...)`
- `ConsumerSpec::read_model(...)`

#### Step 5

If ergonomics matter a lot, add wrapper builders:

- `Automation::new(...)`
- `ReadModel::new(...)`

That gives you immediate value without overcommitting to a more abstract registry system too early.

### 16. My opinion on API shape

If your users are mostly application developers writing slices, I would optimize for this style:

~~~rs
Automation::new("billing", "send-payment-reminder", SendPaymentReminder::new())
    .max_concurrency(32)

ReadModel::new("billing", "invoice-read-model", InvoiceReadModel::new())
~~~

This is very clear.

It says:

- what kind of consumer this is
- what it is called
- what handler it runs
- what execution behavior it needs

That is much easier to reason about than a pair of low-level `durable_subscribe` and `start_automation` primitives.

### Final recommendation

My strongest recommendation is:

- expose `Automation` and `ReadModel` as slice-facing declaration concepts
- normalize both into one internal `ConsumerSpec`
- run everything through one shared runtime pipeline
- keep NATS subscription, shutdown, acking, and task management entirely infrastructure-owned

That gives you:

- semantic clarity
- low startup boilerplate
- clean vertical slice declarations
- room for retries, mirrors, partitioning, and observability later

If you want, next I can turn this into a more concrete draft API that matches your current `NatsStore` and `Project` traits closely, using your existing types and naming style rather than pseudo-infrastructure code.


## Request: Document the event_modeling workflow and retained design decisions

### Summary

The current implementation now has enough surface area that it should be documented as an explicit workflow, not only as a design discussion.

The important thing to capture is that the crate now has a real split between declaration and runtime responsibilities:

- vertical slices declare consumers through `event_modeling`
- infrastructure executes those declarations through `NatsStore`

### Declaration layer

The declaration layer is owned by slices and should stay focused on intent.

That layer currently consists of:

- `ConsumerName`
- `ConsumerSpec<P>`
- `Automation<P>`
- `ReadModel<P>`

A slice should declare:

- the projector
- the semantic role
- the stable structured name
- any execution override

It should not deal with:

- durable consumer creation
- graceful shutdown wiring
- message stream management
- transport-specific subscription mechanics

### Runtime layer

The runtime layer is owned by infrastructure.

That layer currently includes the `NatsStore` consumer entrypoints and helpers that:

- derive durable names from `ConsumerName`
- derive event subjects from the projector event group
- create the durable consumer
- run the sequential or concurrent processing path
- own task spawning and background lifecycle

This preserves the original design goal from the earlier discussion, which was to keep NATS details out of the vertical slice API.

### Why the explicit role wrappers still matter

It is good that the implementation kept both:

- explicit role wrappers
  - `Automation<P>`
  - `ReadModel<P>`
- normalized internal representation
  - `ConsumerSpec<P>`

That gives a cleaner slice-facing API while still preserving one shared runtime execution path.

This was the right compromise from the design discussion because it keeps declarations intention-revealing without duplicating runtime machinery.

### Naming model to document clearly

The naming model should be documented explicitly.

`ConsumerName` currently captures:

- bounded context
- domain
- feature
- consumer

And produces a durable name shaped like:

- `bounded_context.domain.feature.consumer`

That is an important design decision because it replaces ad hoc string naming with a stable operational identity.

### Which dev chat choices were carried forward

The implementation should be documented as following the practical path from the earlier design discussion, especially:

- keeping durable subscription creation private to infrastructure
- using one shared runtime entrypoint
- keeping a shared processing pipeline
- exposing wrapper builders for ergonomics

It should also explicitly note that the initial implementation intentionally skipped the intermediate `ConsumerSpec::automation(...)` and `ConsumerSpec::read_model(...)` constructor step.

That is useful context because the current API shape is centered on the wrapper builders rather than constructor methods on `ConsumerSpec`.

### Recommendation for the docs step

I would document this in a small focused guide that explains:

- declaration layer vs runtime layer
- why `Automation` and `ReadModel` are explicit
- how structured naming works
- which dev chat decisions became implementation choices

That documentation will make the new surface easier to use and will preserve the reasoning behind the current API shape for later refinements.


## Request: Validate the updated projector model after removing DynProject

### Summary

The generic projector execution model is the right long-term fit for the current consumer runtime.

The important result is that the declaration model did not need to change in order to remove `DynProject`.

What changed is only the execution boundary inside infrastructure:

- declaration types remain generic over `P`
- `NatsStore` now owns the typed execution path directly
- sequential and concurrent execution still work with the same declaration API

### What was validated

The retained integration points are still coherent:

- `ConsumerSpec<P>` remains the normalized declaration type
- `Automation<P>` and `ReadModel<P>` still provide the intended slice-facing ergonomics
- durable naming still comes from `ConsumerName`
- subject derivation still comes from `P::EventGroup`
- execution policy still selects between sequential and bounded concurrent processing

That means the original declaration-layer versus runtime-layer split still holds after the projector abstraction change.

### Why the generic projector model is better than `DynProject`

Replacing `DynProject` with generic execution was the correct move because the runtime does not actually need per-message dynamic dispatch over an erased typed context boundary.

The runtime needs to know only:

- which event names belong to the projector event group
- how to own or clone the projector value
- how to invoke `Project::project` once a typed `Context` has been created

Those are all a natural fit for ordinary generic bounds on `P`.

The earlier `DynProject` direction pushed object safety, associated types, and async execution into one erased abstraction, but that abstraction did not correspond to the true runtime boundary.

### Retained design decisions to document

The final workflow should explicitly document that the implementation kept these earlier decisions:

- slice-facing declarations stay explicit through `Automation<P>` and `ReadModel<P>`
- runtime execution is centralized in `NatsStore`
- one shared processing pipeline is used for envelope conversion, context creation, projector execution, error mapping, and acking
- the implementation intentionally kept the wrapper-builder API and still skipped the intermediate `ConsumerSpec` convenience constructor step

### Documentation recommendation

The most useful documentation shape is a focused workflow guide that explains:

- declaration layer versus runtime layer
- why `Automation<P>` and `ReadModel<P>` are still explicit
- how structured naming works
- why generic projector execution replaced `DynProject`
- how sequential and concurrent execution remain supported without changing the declaration model

That would preserve the reasoning behind the current API shape and make the new execution model much easier to understand later.
