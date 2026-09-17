//! What a human looks at: the queue, ordered by how close each ticket is to
//! breaching, and the timeline of one ticket.

use anyhow::Result;
use chrono::{DateTime, Utc};

use super::{
    MilestoneState, QueueEntry, SlaPolicy, Ticket, TicketEvent, TicketPriority, TicketStatus,
};
use super::normalize::normalize_ticket;
use super::sla::compute_sla;

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
            .then_with(|| right.ticket.priority.cmp(&left.ticket.priority))
            .then_with(|| left.next_deadline.cmp(&right.next_deadline))
            .then_with(|| left.ticket.created_at.cmp(&right.ticket.created_at))
            .then_with(|| left.ticket.id.cmp(&right.ticket.id))
    });
    Ok(queue)
}

pub(crate) fn priority_name(priority: TicketPriority) -> &'static str {
    match priority {
        TicketPriority::Low => "low",
        TicketPriority::Normal => "normal",
        TicketPriority::High => "high",
        TicketPriority::Urgent => "urgent",
    }
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
