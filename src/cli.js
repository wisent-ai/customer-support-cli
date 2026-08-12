#!/usr/bin/env node

import { readFile } from 'node:fs/promises'
import { buildQueue, buildTimeline, computeSla, normalizeTicket, routeTicket } from './index.js'

function usage() {
  return `customer-support-cli

Usage:
  customer-support normalize --ticket <ticket.json>
  customer-support queue --tickets <tickets.json> --policy <sla-policy.json> [--at <ISO-8601>] [--include-resolved]
  customer-support sla --ticket <ticket.json> --policy <sla-policy.json> [--at <ISO-8601>]
  customer-support route --ticket <ticket.json> --rules <routing-rules.json>
  customer-support timeline --ticket <ticket.json>

All routing and SLA decisions are deterministic and printed as JSON.`
}

function value(args, name, required = true) {
  const index = args.indexOf(name)
  const result = index >= 0 ? args[index + 1] : null
  if (required && !result) throw new Error(`${name} is required`)
  return result
}

async function json(path) {
  return JSON.parse(await readFile(path, 'utf8'))
}

async function main() {
  const args = process.argv.slice(2)
  if (!args.length || args.includes('--help') || args.includes('-h')) {
    console.log(usage())
    return
  }

  const command = args[0]
  let output
  if (command === 'normalize') {
    output = normalizeTicket(await json(value(args, '--ticket')))
  } else if (command === 'queue') {
    output = buildQueue(
      await json(value(args, '--tickets')),
      await json(value(args, '--policy')),
      { at: value(args, '--at', false), includeResolved: args.includes('--include-resolved') },
    )
  } else if (command === 'sla') {
    output = computeSla(
      await json(value(args, '--ticket')),
      await json(value(args, '--policy')),
      { at: value(args, '--at', false) },
    )
  } else if (command === 'route') {
    output = routeTicket(
      await json(value(args, '--ticket')),
      await json(value(args, '--rules')),
    )
  } else if (command === 'timeline') {
    output = buildTimeline(await json(value(args, '--ticket')))
  } else {
    throw new Error(`Unknown command: ${command}\n\n${usage()}`)
  }
  console.log(JSON.stringify(output, null, 2))
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error))
  process.exitCode = 1
})
