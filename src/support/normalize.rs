//! Reading a ticket as given: the fields it must carry, the dates it
//! declares, and the tags it may hold.

use anyhow::{Context, Result, bail};
use chrono::{DateTime, SecondsFormat, Utc};

use std::collections::BTreeSet;

use super::Ticket;

pub(crate) fn required(value: &str, label: &str) -> Result<String> {
    let normalized = value.trim();
    if normalized.is_empty() {
        bail!("{label} must be a non-empty string");
    }
    Ok(normalized.to_owned())
}

pub(crate) fn optional(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let normalized = item.trim();
        (!normalized.is_empty()).then(|| normalized.to_owned())
    })
}

pub(crate) fn parse_date(value: &str, label: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("{label} must be a valid ISO-8601 date"))
        .map(|date| date.with_timezone(&Utc))
}

pub(crate) fn format_date(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub(crate) fn normalize_tags(tags: Vec<String>) -> Result<Vec<String>> {
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
pub fn parse_optional_date(value: Option<&str>, label: &str) -> Result<Option<DateTime<Utc>>> {
    value.map(|item| parse_date(item, label)).transpose()
}
