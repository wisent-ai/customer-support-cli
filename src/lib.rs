use std::collections::BTreeSet;

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

const PRIORITIES: &[TicketPriority] = &[
    TicketPriority::Low,
    TicketPriority::Normal,
    TicketPriority::High,
    TicketPriority::Urgent,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

impl TicketPriority {
    fn rank(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::High => 2,
            Self::Urgent => 3,
        }
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

fn required(value: &str, label: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        bail!("{label} must be a non-empty string");
    }
    Ok(normalized.to_owned())
}

fn optional(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let normalized = item.trim();
        (!normalized.is_empty()).then(|| normalized.to_owned())
    })
}

fn parse_date(value: &str, label: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("{label} must be a valid ISO-8601 date"))
        .map(|date| date.with_timezone(&Utc))
}

fn format_date(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn normalize_tags(tags: Vec<String>) -> Result<Vec<String>> {
    let mut normalized = BTreeSet::new();
    for tag in tags {
        normalized.insert(required(&tag, "tag")?.to_lowercase());
    }
    Ok(normalized.into_iter().collect())
}

pub fn normalize_ticket(mut ticket: Ticket) -> Result<Ticket> {
    ticket.id = required(&ticket.id, "ticket.id")?;
    ticket.subject = optional(ticket.subject);
    ticket.created_at = format_date(parse_date(&ticket.created_at, "ticket.createdAt")?);
    ticket.updated_at = Some(format_date(parse_date(
        ticket.updated_at.as_deref().unwrap_or(&ticket.created_at),
        "ticket.updatedAt",
    )?));
    ticket.channel = optional(ticket.channel);
    ticket.requester_id = optional(ticket.requester_id);
    ticket.assignee = optional(ticket.assignee);
    ticket.team = optional(ticket.team);
    ticket.product_area = optional(ticket.product_area);
    ticket.tags = normalize_tags(ticket.tags)?;
    for (index, event) in ticket.events.iter_mut().enumerate() {
        event.kind = required(&event.kind, &format!("ticket.events[{index}].type"))?;
        event.at = format_date(parse_date(
            &event.at,
            &format!("ticket.events[{index}].at"),
        )?);
        event.actor = optional(event.actor.take());
        event.body = optional(event.body.take());
        event.visibility = required(
            &event.visibility,
            &format!("ticket.events[{index}].visibility"),
        )?;
    }
    ticket.events.sort_by(|left, right| left.at.cmp(&right.at));
    Ok(ticket)
}

pub fn validate_policy(policy: &SlaPolicy) -> Result<()> {
    for priority in PRIORITIES {
        let rule = policy.rule(*priority);
        if !rule.first_response_hours.is_finite() || rule.first_response_hours <= 0.0 {
            bail!("firstResponseHours must be greater than zero");
        }
        if !rule.resolution_hours.is_finite() || rule.resolution_hours <= 0.0 {
            bail!("resolutionHours must be greater than zero");
        }
    }
    Ok(())
}

fn add_hours(start: DateTime<Utc>, hours: f64) -> Result<DateTime<Utc>> {
    let milliseconds = hours * 3_600_000.0;
    if !milliseconds.is_finite() || milliseconds > i64::MAX as f64 {
        bail!("SLA duration is too large");
    }
    Ok(start + Duration::milliseconds(milliseconds.round() as i64))
}

fn milestone(
    due_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> SlaMilestone {
    let state = match completed_at {
        Some(completed) if completed <= due_at => MilestoneState::Met,
        Some(_) => MilestoneState::Breached,
        None if now <= due_at => MilestoneState::Pending,
        None => MilestoneState::Breached,
    };
    SlaMilestone {
        due_at: format_date(due_at),
        completed_at: completed_at.map(format_date),
        state,
    }
}

pub fn compute_sla(ticket: &Ticket, policy: &SlaPolicy, at: DateTime<Utc>) -> Result<TicketSla> {
    validate_policy(policy)?;
    let created = parse_date(&ticket.created_at, "ticket.createdAt")?;
    let rule = policy.rule(ticket.priority);
    let first_reply = ticket
        .events
        .iter()
        .find(|event| event.kind == "agent_reply" && event.visibility == "public")
        .map(|event| parse_date(&event.at, "event.at"))
        .transpose()?;
    let resolution = if matches!(ticket.status, TicketStatus::Resolved | TicketStatus::Closed) {
        match ticket
            .events
            .iter()
            .rev()
            .find(|event| event.kind == "resolved")
        {
            Some(event) => Some(parse_date(&event.at, "event.at")?),
            None => ticket
                .updated_at
                .as_deref()
                .map(|value| parse_date(value, "ticket.updatedAt"))
                .transpose()?,
        }
    } else {
        None
    };
    Ok(TicketSla {
        ticket_id: ticket.id.clone(),
        evaluated_at: format_date(at),
        first_response: milestone(
            add_hours(created, rule.first_response_hours)?,
            first_reply,
            at,
        ),
        resolution: milestone(add_hours(created, rule.resolution_hours)?, resolution, at),
    })
}

pub fn build_queue(
    inputs: Vec<Ticket>,
    policy: &SlaPolicy,
    at: DateTime<Utc>,
    include_resolved: bool,
) -> Result<Vec<QueueEntry>> {
    let mut queue = Vec::with_capacity(inputs.len());
    for input in inputs {
        let ticket = normalize_ticket(input)?;
        if !include_resolved
            && matches!(ticket.status, TicketStatus::Resolved | TicketStatus::Closed)
        {
            continue;
        }
        let sla = compute_sla(&ticket, policy, at)?;
        let breached = matches!(sla.first_response.state, MilestoneState::Breached)
            || matches!(sla.resolution.state, MilestoneState::Breached);
        let next_deadline = if sla.first_response.completed_at.is_none() {
            sla.first_response.due_at.clone()
        } else {
            sla.resolution.due_at.clone()
        };
        queue.push(QueueEntry {
            ticket,
            sla,
            breached,
            next_deadline,
        });
    }
    queue.sort_by(|left, right| {
        right
            .breached
            .cmp(&left.breached)
            .then_with(|| {
                right
                    .ticket
                    .priority
                    .rank()
                    .cmp(&left.ticket.priority.rank())
            })
            .then_with(|| left.next_deadline.cmp(&right.next_deadline))
            .then_with(|| left.ticket.created_at.cmp(&right.ticket.created_at))
            .then_with(|| left.ticket.id.cmp(&right.ticket.id))
    });
    Ok(queue)
}

fn priority_name(priority: TicketPriority) -> &'static str {
    match priority {
        TicketPriority::Low => "low",
        TicketPriority::Normal => "normal",
        TicketPriority::High => "high",
        TicketPriority::Urgent => "urgent",
    }
}

pub fn route_ticket(ticket: &Ticket, rules: Vec<RoutingRule>) -> Result<RoutingDecision> {
    for (index, rule) in rules.into_iter().enumerate() {
        let required_tags = normalize_tags(rule.criteria.tags.clone())?;
        let matches = rule
            .criteria
            .channel
            .as_ref()
            .is_none_or(|value| value.matches(ticket.channel.as_deref()))
            && rule
                .criteria
                .priority
                .as_ref()
                .is_none_or(|value| value.matches(Some(priority_name(ticket.priority))))
            && rule
                .criteria
                .product_area
                .as_ref()
                .is_none_or(|value| value.matches(ticket.product_area.as_deref()))
            && required_tags.iter().all(|tag| ticket.tags.contains(tag));
        if matches {
            return Ok(RoutingDecision {
                ticket_id: ticket.id.clone(),
                rule_id: Some(required(&rule.id, &format!("rules[{index}].id"))?),
                team: Some(required(&rule.team, &format!("rules[{index}].team"))?),
            });
        }
    }
    Ok(RoutingDecision {
        ticket_id: ticket.id.clone(),
        rule_id: None,
        team: None,
    })
}

pub fn build_timeline(ticket: &Ticket) -> Vec<TicketEvent> {
    let mut timeline = Vec::with_capacity(ticket.events.len() + 1);
    timeline.push(TicketEvent {
        kind: "created".to_owned(),
        at: ticket.created_at.clone(),
        actor: ticket.requester_id.clone(),
        body: ticket.subject.clone(),
        visibility: "public".to_owned(),
    });
    timeline.extend(ticket.events.clone());
    timeline.sort_by(|left, right| left.at.cmp(&right.at));
    timeline
}

pub fn parse_optional_date(value: Option<&str>, label: &str) -> Result<Option<DateTime<Utc>>> {
    value.map(|item| parse_date(item, label)).transpose()
}
