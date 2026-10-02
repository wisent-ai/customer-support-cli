<!-- wisent-banner:start -->
<p align="center">
  <img src="assets/readme-banner.webp" alt="customer-support-cli by Wisent" width="100%">
</p>
<!-- wisent-banner:end -->

<!-- wisent-readme-signals:start -->
[![Source](https://img.shields.io/badge/GitHub-Source-181717?logo=github)](https://github.com/wisent-ai/customer-support-cli) [![Issues](https://img.shields.io/badge/GitHub-Issues-181717?logo=github)](https://github.com/wisent-ai/customer-support-cli/issues) [![Wisent](https://img.shields.io/badge/Wisent-Website-0B0B0B)](https://wisent.com) [![Discord](https://img.shields.io/badge/Discord-Join-5865F2?logo=discord&logoColor=white)](https://discord.gg/qRjpkthq54) [![LinkedIn](https://img.shields.io/badge/LinkedIn-Follow-0A66C2?logo=linkedin&logoColor=white)](https://www.linkedin.com/company/wisent-ai/) [![X](https://img.shields.io/badge/X-Follow-000000?logo=x&logoColor=white)](https://x.com/wisentai) [![Enterprise](https://img.shields.io/badge/Enterprise-Book%20a%20call-0B0B0B?logo=calendly)](https://calendly.com/lbartoszcze)
<!-- wisent-readme-signals:end -->

# customer-support-cli

Deterministic triage for support tickets: canonical form, SLA, routing,
queue order and timeline. Every command reads JSON files and prints JSON, or
with `--text` one `path: value` line per field from the same document.

| Command | What it prints |
|---|---|
| `customer-support normalize --ticket <ticket.json>` | the ticket in canonical form |
| `customer-support sla --ticket <ticket.json> --policy <policy.json> [--at <RFC 3339>]` | the ticket's SLA targets and whether they are met |
| `customer-support route --ticket <ticket.json> --rules <rules.json>` | the team the first matching rule sends it to |
| `customer-support queue --tickets <tickets.json> --policy <policy.json> [--at <RFC 3339>] [--include-resolved]` | open tickets ordered by how close each is to breaching |
| `customer-support timeline --ticket <ticket.json>` | the ticket's events in time order |

`--at` defaults to now. `--help` works on the program and on every command.
Exit 2: the invocation is wrong (unknown command or flag, missing argument);
exit 1: an input file could not be read, is not valid JSON, or was refused,
with the reason on stderr.
