
# Knowledge Files Descriptions (Sent to AI for selection)

- docs/skill/esrc-slice-constants-and-module-layout.md
    - Summary: Defines the standards for vertical slice structures, folder layouts, and required constants (BOUNDED_CONTEXT_NAME, DOMAIN_NAME, FEATURE_NAME) within an esrc-based project to ensure isolation and consistent instrumentation.
    - When To Use: Use this guide when creating new feature slices, organizing domain modules, or setting up the module hierarchy in a Rust project following esrc architecture patterns.

- docs/skill/esrc-event-modeling-create-consumers-automations-and-read-models.md
    - Summary: This document provides guidance on declaring event-driven consumers (automations and read models) using the esrc::event_modeling library, focusing on naming standards, execution policies, and the Project trait implementation.
    - When To Use: Use this guide when implementing reactive side effects (automations) or materializing queryable state (read models) within a vertical slice, ensuring correct consumer naming and concurrency configurations.
    - Types: ConsumerName, ConsumerRole, ExecutionPolicy, ConsumerSpec, Automation, ReadModel, Project, Context
    - Functions: ConsumerName::new, Automation::new, ReadModel::new, max_concurrency, with_execution_policy, into_spec, project

- docs/skill/esrc-command-service-execute-commands.md
    - Summary: Explains how to use the `esrc::event::command_service::CommandClient` to trigger aggregate state changes from a slice, detailing command execution logic, error mapping, and best practices for isolation and idempotency.
    - When To Use: When building vertical slices that need to send commands to aggregates, handle command results (like conflicts or domain errors), or implement event-driven automations.
    - Types: CommandClient, Aggregate
    - Functions: send_command

- docs/skill/esrc-read-model-public-interface-and-queries.md
    - Summary: Defines the standard pattern for structuring read model public interfaces and queries within a slice, using a generated.rs for data structures and a mod.rs for custom query extensions.
    - When To Use: Include this when designing slice read models, defining public query APIs, or setting up the interaction boundary between different slices and their consumers.
    - Types: Queries (enum), generated.rs (structs), RedeQuery (example pattern)

