# chirpy
Chirpy is apalis in standalone server mode — a single deployable binary that any service can push jobs to, regardless of language.

**Chirpy is currently in active development. APIs, protocols, and features described on this page reflect the planned design and are subject to change. Follow the [GitHub repository](https://github.com/apalis-dev/chirpy) for progress updates.**

Apalis works exceptionally well as an embedded library — your Rust application imports it, declares workers, and runs everything in-process. But not every team wants to embed a job engine into every service. In a multi-service environment, that means multiple independent job queues, disconnected dashboards, and duplicated operational overhead.

**Chirpy** is the answer to that problem. It is a standalone binary built on Apalis that runs the job engine as a central server. Any service in your infrastructure — Rust, Go, Python, JavaScript, or anything else that speaks RESP, HTTP or RPC — can push jobs to it, and a fleet of workers can pull from it, all through a single deployment.

## What Chirpy Provides

- **A standalone binary** — deploy it like any other service: Docker, systemd, Kubernetes
- **A language-agnostic API** — push and query jobs over HTTP or gRPC from any language
- **All Apalis capabilities** — retries, scheduled jobs, priority queues, heartbeating, and failure queues, backed by your choice of PostgreSQL, SQLite, or Redis
- **A built-in web dashboard** — the full `apalis-board` interface, served directly from the binary
- **A metrics endpoint** — Prometheus-compatible `/metrics` out of the box

## Comparison with Apalis

| | Apalis | Chirpy |
|---|---|---|
| Deployment | Inside your application | Standalone binary |
| Language support | Rust only | Any language via HTTP/gRPC |
| Worker location | Same process as the server | Anywhere on the network |
| Operational overhead | Per-service | Centralised |
| Dashboard | Optional, self-hosted | Built in |
| Best for | Single-service Rust apps | Multi-service or polyglot environments |

Both modes will continue to be supported. Chirpy is an addition to the ecosystem, not a replacement for embedded Apalis.

## Architecture
```
┌─────────────────────────────────────────────────────┐
│                    Chirpy Server                    │
│                                                     │
│  ┌─────────────┐   ┌────────────┐  ┌─────────────┐ │
│  │  HTTP API   │   │ gRPC API   │  │  Web UI     │ │
│  │  /jobs      │   │ JobService │  │  /dashboard │ │
│  └──────┬──────┘   └─────┬──────┘  └─────────────┘ │
│         │                │                          │
│         └────────┬───────┘                          │
│                  ▼                                   │
│         ┌─────────────────┐                         │
│         │   Apalis Core   │                         │
│         │  Backend + Pool │                         │
│         └────────┬────────┘                         │
│                  │                                   │
│         ┌────────▼────────┐                         │
│         │    Storage      │                         │
│         │ PG / SQLite /   │                         │
│         │ Redis           │                         │
│         └─────────────────┘                         │
└─────────────────────────────────────────────────────┘
         ▲               ▲               ▲
         │               │               │
   Rust client     Python client    Go client
   (native SDK)    (HTTP)           (gRPC)
```

Chirpy exposes two API surfaces over the same underlying Apalis backend. Push a job from a Python service over HTTP, pull it from a Rust worker using the native SDK, and watch it all from the web dashboard — the backend does not care which client pushed which job.


## Rust client

Rust applications can connect a standard Apalis worker directly to a running Chirpy instance using the `chirpy-client` crate:
```rust
use apalis::prelude::*;
use chirpy_client::ChirpyBackend;

async fn send_email(email: Email) -> Result<(), BoxDynError> {
    // handler logic
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), BoxDynError> {
    let backend = ChirpyBackend::connect("http://chirpy.internal:7000")
        .queue("email")
        .await?;

    WorkerBuilder::new("email-worker")
        .backend(backend)
        .enable_tracing()
        .build(send_email)
        .run()
        .await?;

    Ok(())
}
```

## Deployment

Chirpy is distributed as a single static binary with no runtime dependencies beyond the chosen storage backend.

### Docker
```bash
docker run -e DATABASE_URL=postgres://user:pass@localhost/chirpy \
           -p 7000:7000 \
           ghcr.io/apalis-dev/chirpy:latest
```

## Roadmap

Chirpy is under active development. The planned milestones are:

### Alpha — Core Protocol
- [ ] RESP API for push, ack, fetch and list
- [ ] HTTP API for push, ack, list, and inspect
- [ ] PostgreSQL and SQLite storage backends
- [ ] `chirpy-client` Rust crate with `Backend` implementation
- [ ] Basic web dashboard via `apalis-board`
- [ ] Docker image and basic deployment documentation

### Beta — Extended Capabilities
- [ ] gRPC API and published `.proto` definitions
- [ ] Redis storage backend
- [ ] Prometheus `/metrics` endpoint
- [ ] Scheduled and delayed job support via API
- [ ] Priority queue support
- [ ] Official Go client SDK

### Stable — Ecosystem
- [ ] Official Python client SDK
- [ ] JavaScript / TypeScript client SDK
- [ ] Multi-tenant queue namespacing
- [ ] API key authentication
- [ ] Horizontal scaling — multiple Chirpy instances against a shared backend
- [ ] Webhook support — push job results to a URL on completion

---

## Relationship to Apalis

Chirpy is built entirely on Apalis internals. It does not introduce a new job engine — it wraps the existing one in a network-accessible shell. This means:

- Every Chirpy deployment benefits from Apalis improvements automatically
- Teams already using embedded Apalis can migrate to Chirpy without changing their worker code — only the backend connection changes
- The `apalis-board` dashboard works identically whether you are using embedded Apalis or Chirpy

---

## Get Involved

Chirpy is in early development and the design is still open. If you have opinions on the protocol, the API shape, or priorities for language SDKs, the best place to contribute is the [GitHub discussion thread](https://github.com/orgs/apalis-dev/discussions) or by opening a pull request against the [Chirpy repository](https://github.com/apalis-dev/chirpy).

---

