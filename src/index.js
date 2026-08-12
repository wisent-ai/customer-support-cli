const PRIORITIES = ['low', 'normal', 'high', 'urgent']
const STATUSES = ['open', 'pending', 'resolved', 'closed']

function requiredString(value, label) {
  const normalized = String(value ?? '').trim()
  if (!normalized) throw new TypeError(`${label} must be a non-empty string`)
  return normalized
}

function optionalString(value) {
  const normalized = String(value ?? '').trim()
  return normalized || null
}

function isoDate(value, label) {
  const date = new Date(value)
  if (!Number.isFinite(date.getTime())) throw new TypeError(`${label} must be a valid date`)
  return date.toISOString()
}

function positiveHours(value, label) {
  const hours = Number(value)
  if (!Number.isFinite(hours) || hours <= 0) throw new TypeError(`${label} must be greater than zero`)
  return hours
}

function normalizeTags(value) {
  if (!Array.isArray(value)) return []
  return [...new Set(value.map((tag) => requiredString(tag, 'tag')).map((tag) => tag.toLowerCase()))].sort()
}

function normalizeEvent(event, index) {
  if (!event || typeof event !== 'object' || Array.isArray(event)) throw new TypeError(`ticket.events[${index}] must be an object`)
  return {
    type: requiredString(event.type, `ticket.events[${index}].type`),
    at: isoDate(event.at, `ticket.events[${index}].at`),
    actor: optionalString(event.actor),
    body: optionalString(event.body),
    visibility: optionalString(event.visibility) ?? 'public',
  }
}

export function normalizeTicket(input) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new TypeError('ticket must be an object')
  const priority = requiredString(input.priority ?? 'normal', 'ticket.priority').toLowerCase()
  const status = requiredString(input.status ?? 'open', 'ticket.status').toLowerCase()
  if (!PRIORITIES.includes(priority)) throw new TypeError(`unsupported ticket priority: ${priority}`)
  if (!STATUSES.includes(status)) throw new TypeError(`unsupported ticket status: ${status}`)

  const events = Array.isArray(input.events) ? input.events.map(normalizeEvent) : []
  events.sort((left, right) => left.at.localeCompare(right.at))
  return {
    id: requiredString(input.id, 'ticket.id'),
    subject: optionalString(input.subject),
    createdAt: isoDate(input.createdAt, 'ticket.createdAt'),
    updatedAt: isoDate(input.updatedAt ?? input.createdAt, 'ticket.updatedAt'),
    priority,
    status,
    channel: optionalString(input.channel),
    requesterId: optionalString(input.requesterId),
    assignee: optionalString(input.assignee),
    team: optionalString(input.team),
    productArea: optionalString(input.productArea),
    tags: normalizeTags(input.tags),
    events,
  }
}

export function normalizeSlaPolicy(input) {
  if (!input || typeof input !== 'object' || Array.isArray(input)) throw new TypeError('policy must be an object')
  const priorities = {}
  for (const priority of PRIORITIES) {
    const rule = input.priorities?.[priority]
    if (!rule || typeof rule !== 'object' || Array.isArray(rule)) throw new TypeError(`policy.priorities.${priority} is required`)
    priorities[priority] = {
      firstResponseHours: positiveHours(rule.firstResponseHours, `policy.priorities.${priority}.firstResponseHours`),
      resolutionHours: positiveHours(rule.resolutionHours, `policy.priorities.${priority}.resolutionHours`),
    }
  }
  return { priorities }
}

function deadline(createdAt, hours) {
  return new Date(new Date(createdAt).getTime() + hours * 60 * 60 * 1000).toISOString()
}

function milestoneState(dueAt, completedAt, now) {
  if (completedAt) return new Date(completedAt) <= new Date(dueAt) ? 'met' : 'breached'
  return new Date(now) <= new Date(dueAt) ? 'pending' : 'breached'
}

export function computeSla(ticketInput, policyInput, options = {}) {
  const ticket = normalizeTicket(ticketInput)
  const policy = normalizeSlaPolicy(policyInput)
  const at = isoDate(options.at ?? new Date(), 'options.at')
  const rule = policy.priorities[ticket.priority]
  const firstReply = ticket.events.find((event) => event.type === 'agent_reply' && event.visibility === 'public')?.at ?? null
  const resolution = ticket.status === 'resolved' || ticket.status === 'closed'
    ? [...ticket.events].reverse().find((event) => event.type === 'resolved')?.at ?? ticket.updatedAt
    : null
  const firstResponseDueAt = deadline(ticket.createdAt, rule.firstResponseHours)
  const resolutionDueAt = deadline(ticket.createdAt, rule.resolutionHours)

  return {
    ticketId: ticket.id,
    evaluatedAt: at,
    firstResponse: {
      dueAt: firstResponseDueAt,
      completedAt: firstReply,
      state: milestoneState(firstResponseDueAt, firstReply, at),
    },
    resolution: {
      dueAt: resolutionDueAt,
      completedAt: resolution,
      state: milestoneState(resolutionDueAt, resolution, at),
    },
  }
}

function nextDeadline(sla) {
  if (!sla.firstResponse.completedAt) return sla.firstResponse.dueAt
  return sla.resolution.dueAt
}

export function buildQueue(ticketInputs, policyInput, options = {}) {
  if (!Array.isArray(ticketInputs)) throw new TypeError('tickets must be an array')
  const includeResolved = options.includeResolved === true
  const queue = ticketInputs.map((input) => {
    const ticket = normalizeTicket(input)
    const sla = computeSla(ticket, policyInput, options)
    return {
      ticket,
      sla,
      breached: sla.firstResponse.state === 'breached' || sla.resolution.state === 'breached',
      nextDeadline: nextDeadline(sla),
    }
  }).filter((entry) => includeResolved || !['resolved', 'closed'].includes(entry.ticket.status))

  return queue.sort((left, right) =>
    Number(right.breached) - Number(left.breached)
    || PRIORITIES.indexOf(right.ticket.priority) - PRIORITIES.indexOf(left.ticket.priority)
    || left.nextDeadline.localeCompare(right.nextDeadline)
    || left.ticket.createdAt.localeCompare(right.ticket.createdAt)
    || left.ticket.id.localeCompare(right.ticket.id))
}

function exactMatch(actual, expected) {
  if (expected === undefined) return true
  if (Array.isArray(expected)) return expected.map(String).includes(String(actual))
  return String(actual) === String(expected)
}

export function routeTicket(ticketInput, ruleInputs) {
  const ticket = normalizeTicket(ticketInput)
  if (!Array.isArray(ruleInputs)) throw new TypeError('rules must be an array')
  for (const [index, rule] of ruleInputs.entries()) {
    if (!rule || typeof rule !== 'object' || Array.isArray(rule)) throw new TypeError(`rules[${index}] must be an object`)
    const matches = rule.match ?? {}
    const requiredTags = normalizeTags(matches.tags)
    if (
      exactMatch(ticket.channel, matches.channel)
      && exactMatch(ticket.priority, matches.priority)
      && exactMatch(ticket.productArea, matches.productArea)
      && requiredTags.every((tag) => ticket.tags.includes(tag))
    ) {
      return {
        ticketId: ticket.id,
        ruleId: requiredString(rule.id, `rules[${index}].id`),
        team: requiredString(rule.team, `rules[${index}].team`),
      }
    }
  }
  return { ticketId: ticket.id, ruleId: null, team: null }
}

export function buildTimeline(ticketInput) {
  const ticket = normalizeTicket(ticketInput)
  return [
    { type: 'created', at: ticket.createdAt, actor: ticket.requesterId, body: ticket.subject, visibility: 'public' },
    ...ticket.events,
  ].sort((left, right) => left.at.localeCompare(right.at))
}
