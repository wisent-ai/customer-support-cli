//! Routing a ticket: the first rule whose match applies, and why.

use anyhow::Result;

use super::{RoutingDecision, RoutingRule, Ticket};
use super::normalize::{normalize_tags, required};
use super::queue::priority_name;

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
