//! Support tickets: one shape for a ticket, the SLA it is measured against,
//! the queue a human works from, and where a ticket is routed.
//!
//! The shapes and the operations live in `support`; this file is the surface
//! the CLI and any other caller uses, so the crate's API is one list.

pub mod support;

pub use support::{
    MatchValue, MilestoneState, QueueEntry, RoutingDecision, RoutingMatch, RoutingRule,
    SlaMilestone, SlaPolicy, SlaPriorities, SlaRule, Ticket, TicketEvent, TicketPriority,
    TicketSla, TicketStatus,
};
pub use support::normalize::{normalize_ticket, parse_optional_date};
pub use support::queue::{build_queue, build_timeline};
pub use support::routing::route_ticket;
pub use support::sla::{compute_sla, validate_policy};
