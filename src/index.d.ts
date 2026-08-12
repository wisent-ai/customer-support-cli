export type TicketPriority = 'low' | 'normal' | 'high' | 'urgent'
export type TicketStatus = 'open' | 'pending' | 'resolved' | 'closed'

export interface TicketEvent {
  type: string
  at: string | Date
  actor?: string | null
  body?: string | null
  visibility?: string
}

export interface Ticket {
  id: string
  subject?: string | null
  createdAt: string | Date
  updatedAt?: string | Date
  priority?: TicketPriority
  status?: TicketStatus
  channel?: string | null
  requesterId?: string | null
  assignee?: string | null
  team?: string | null
  productArea?: string | null
  tags?: string[]
  events?: TicketEvent[]
}

export interface SlaPolicy {
  priorities: Record<TicketPriority, { firstResponseHours: number; resolutionHours: number }>
}

export interface SlaMilestone {
  dueAt: string
  completedAt: string | null
  state: 'pending' | 'met' | 'breached'
}

export interface TicketSla {
  ticketId: string
  evaluatedAt: string
  firstResponse: SlaMilestone
  resolution: SlaMilestone
}

export interface RoutingRule {
  id: string
  team: string
  match?: {
    channel?: string | string[]
    priority?: TicketPriority | TicketPriority[]
    productArea?: string | string[]
    tags?: string[]
  }
}

export function normalizeTicket(input: Ticket): Ticket
export function normalizeSlaPolicy(input: SlaPolicy): SlaPolicy
export function computeSla(ticket: Ticket, policy: SlaPolicy, options?: { at?: string | Date | null }): TicketSla
export function buildQueue(tickets: Ticket[], policy: SlaPolicy, options?: { at?: string | Date | null; includeResolved?: boolean }): Array<{ ticket: Ticket; sla: TicketSla; breached: boolean; nextDeadline: string }>
export function routeTicket(ticket: Ticket, rules: RoutingRule[]): { ticketId: string; ruleId: string | null; team: string | null }
export function buildTimeline(ticket: Ticket): TicketEvent[]
