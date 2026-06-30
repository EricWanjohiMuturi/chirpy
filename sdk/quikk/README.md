# Quikk

The official Python SDK for **Chirpy**, a high-performance, language-agnostic standalone job server built on top of [apalis](https://apalis.dev).

`Quikk` brings Rust-level job processing reliability to the Python ecosystem with a focus on **Type Safety**, **Developer Experience**, and **Performance**.

> Formerly published under the working name `apalis-py`.

---

## Key Features
* **RESP Protocol:** Fast, binary-safe communication with the Chirpy server.
* **Type-Safe Payloads:** Built-in Pydantic models for job validation. *(Phase 2, in progress)*
* **Async Native:** Designed for `asyncio` and modern web frameworks (FastAPI, etc.), via `anyio`.
* **Hybrid Execution:** Support for both `async def` and standard `def` tasks. *(Phase 5/6, planned)*

## Installation (Coming Soon)
```bash
uv add quikk
# or
pip install quikk
```

## Quick Start (Phase 1 transport layer)
```python
from quikk import Connection

conn = Connection()
await conn.connect("localhost", 9101)
greeting = await conn.read_response()  # SimpleString from the server
await conn.send_raw('HELLO {"v":2}')
hello_resp = await conn.read_response()
await conn.close()
```

## Quikk Roadmap

### Phase 1: RESP Transport Layer — ✅ Complete
The raw I/O foundation everything else sits on. No protocol knowledge at this level — just bytes and lines.
- [x] **TCP Connection:** Persistent TCP socket via `anyio` (supports both asyncio and trio).
- [x] **Buffered Line Reader:** Internal byte buffer fed in chunks from the socket; lines and exact-length reads are served from the buffer instead of one `receive()` call per byte.
- [x] **Response Parser:** Parses all three server response types: Simple Strings (`+`), Errors (`-`), and Bulk Strings (`$<len>\r\n<data>\r\n`), including the Null Bulk String (`$-1\r\n`).
- [x] **Raw Send Primitive:** Writes a command string to the socket with a guaranteed `\r\n` terminator.
- [x] **Public API:** `Connection`, `parse_response`, and all response/error types are importable directly from `quikk`.

### Phase 2: Protocol & Type System ✅ Complete
Map the Chirpy protocol to Python. This layer knows the shape of every command and response.
- [x] **Pydantic Models:** Define `Job`, `Failure`, and all command payload types (`HelloData`, `BeatData`, `FailData`, etc.) mirroring `job.rs` and `command.rs`.
- [x] **Command Serializers:** Python objects → wire-format strings (e.g. `PUSH <json>`, `ACK {"jid":"..."}`, `FETCH ["queue"]`).
- [x] **Authentication:** Implement SCRAM-SHA-256 password hashing (`sha256(password + salt)` iterated `n` times) to support password-protected servers.
- [x] **`Job` Builder:** Fluent interface for constructing jobs with defaults for `queue`, `retry`, and `reserve_for`.

### Phase 3: Connection & Handshake *(next up)*
A managed `Connection` object that speaks the Chirpy protocol on top of Phase 1 transport.
- [ ] **Handshake Orchestration:** On connect, receive `HI`, send `HELLO` (with or without auth), assert `+OK`.
- [ ] **Producer vs. Worker Modes:** `HELLO` payload differs — producers send minimal fields; workers include `wid`, `hostname`, `pid`, `labels`.
- [ ] **Reconnection:** Exponential backoff reconnection with automatic re-handshake on connection loss.
- [ ] **`INFO` Support:** Expose the `INFO` command for health checks and server introspection.

### Phase 4: The Producer (Client)
- [ ] **`ApalisClient`:** High-level client for pushing jobs — the primary interface for web app code.
- [ ] **Scheduled Jobs:** Expose the `at` field so jobs can be deferred to a future time.
- [ ] **Retry Configuration:** Allow setting `retry` count and `backtrace` depth per job.
- [ ] **Custom Metadata:** Expose the `custom` field for tracing IDs, tenant context, or any arbitrary metadata.
- [ ] **Connection Pooling:** Manage a pool of persistent connections to eliminate per-push handshake overhead.
- [ ] **Client-Side Validation:** Validate all job fields with Pydantic before a single byte leaves the process.

### Phase 5: The Consumer (Worker)
- [ ] **`FETCH` Loop:** Long-polling loop that continuously requests jobs from one or more queues.
- [ ] **Background Heartbeat:** Automatic `BEAT` task that pings the server on a fixed interval to maintain worker registration.
- [ ] **Task Dispatcher:** Registry that maps `jobtype` strings to Python handler functions.
- [ ] **`ACK` / `FAIL` Reporting:** Send `ACK` on success; send `FAIL` with `errtype`, `message`, and a formatted traceback on any exception.
- [ ] **Graceful Shutdown:** Handle the `QUIET` → drain in-flight jobs → `END` lifecycle correctly on `SIGTERM`.
- [ ] **Concurrency Control:** Run multiple jobs concurrently using `anyio` task groups with a configurable concurrency limit.

### Phase 6: Developer Experience (DX)
- [ ] **`@quikk.task()` Decorator:** Register handlers with queue and retry options — familiar Celery-like syntax.
- [ ] **Async/Sync Transparency:** Automatically run `def` functions in a thread pool; run `async def` directly.
- [ ] **Middleware / Hooks:** `before_fetch`, `after_ack`, `on_fail` hooks for logging, metrics, and custom error handling.
- [ ] **`quikk run` CLI:** Boot a worker process from the terminal, configurable via env vars or a config file.

### Phase 7: Reliability & Observability
- [ ] **Integration Test Suite:** Automated tests that run against a live Chirpy instance (spin up via subprocess or Docker).
- [ ] **Structured Logging:** Built-in `structlog`-compatible logging with job ID, queue, and worker ID on every event.
- [ ] **Performance Benchmarks:** Measure throughput bottlenecks — RESP parser overhead vs. worker loop vs. task execution.
- [ ] **Examples:** End-to-end integration guides for FastAPI and Django.
