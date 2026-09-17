//! What a support ticket is: its priority and status, the events on it, the
//! SLA policy it is measured against, and the shapes the queue and the
//! router answer with.
//!
//! The operations are the submodules: normalising a ticket, computing its
//! SLA, building the queue, and routing it.

pub mod normalize;
pub mod queue;
pub mod routing;
pub mod sla;

use serde::{Deserialize, Serialize};

pub(crate) const PRIORITIES: &[TicketPriority] = &[
    TicketPriority::Low,
    TicketPriority::Normal,
    TicketPriority::High,
    TicketPriority::Urgent,
];

// Variants are declared from least to most urgent; the derived order is the queue's ranking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TicketPriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl Default for TicketPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TicketStatus {
    Open,
    Pending,
    Resolved,
    Closed,
}

impl Default for TicketStatus {
    fn default() -> Self {
        Self::Open
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketEvent {
    #[serde(rename = "type")]
    pub kind: String,
    pub at: String,
    #[serde(default)]
    pub actor: Option<String>,
    #[serde(default)]
    pub body: Option<String>,
    #[serde(default = "public_visibility")]
    pub visibility: String,
}

fn public_visibility() -> String {
    "public".to_owned()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Ticket {
    pub id: String,
    #[serde(default)]
    pub subject: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub priority: TicketPriority,
    #[serde(default)]
    pub status: TicketStatus,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub requester_id: Option<String>,
    #[serde(default)]
    pub assignee: Option<String>,
    #[serde(default)]
    pub team: Option<String>,
    #[serde(default)]
    pub product_area: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub events: Vec<TicketEvent>,
}
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlaRule {
    pub first_response_hours: f64,
    pub resolution_hours: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlaPriorities {
    pub low: SlaRule,
    pub normal: SlaRule,
    pub high: SlaRule,
    pub urgent: SlaRule,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SlaPolicy {
    pub priorities: SlaPriorities,
}

impl SlaPolicy {
    fn rule(&self, priority: TicketPriority) -> &SlaRule {
        match priority {
            TicketPriority::Low => &self.priorities.low,
            TicketPriority::Normal => &self.priorities.normal,
            TicketPriority::High => &self.priorities.high,
            TicketPriority::Urgent => &self.priorities.urgent,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum MilestoneState {
    Pending,
    Met,
    Breached,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlaMilestone {
    pub due_at: String,
    pub completed_at: Option<String>,
    pub state: MilestoneState,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TicketSla {
    pub ticket_id: String,
    pub evaluated_at: String,
    pub first_response: SlaMilestone,
    pub resolution: SlaMilestone,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueueEntry {
    pub ticket: Ticket,
    pub sla: TicketSla,
    pub breached: bool,
    pub next_deadline: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum MatchValue {
    One(String),
    Many(Vec<String>),
}

impl MatchValue {
    fn matches(&self, actual: Option<&str>) -> bool {
        let actual = actual.unwrap_or_default();
        match self {
            Self::One(expected) => expected == actual,
            Self::Many(expected) => expected.iter().any(|item| item == actual),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingMatch {
    #[serde(default)]
    pub channel: Option<MatchValue>,
    #[serde(default)]
    pub priority: Option<MatchValue>,
    #[serde(default)]
    pub product_area: Option<MatchValue>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RoutingRule {
    pub id: String,
    pub team: String,
    #[serde(default, rename = "match")]
    pub criteria: RoutingMatch,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoutingDecision {
    pub ticket_id: String,
    pub rule_id: Option<String>,
    pub team: Option<String>,
}
