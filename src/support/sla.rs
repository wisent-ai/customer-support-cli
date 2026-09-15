//! The service-level agreement: whether a policy is usable, when each
//! milestone is due, and where a ticket stands against it right now.

use anyhow::{Result, bail};
use chrono::{DateTime, Duration, Utc};

use super::{
    MilestoneState, PRIORITIES, SlaMilestone, SlaPolicy, Ticket, TicketSla, TicketStatus,
};
use super::normalize::{format_date, parse_date};

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
