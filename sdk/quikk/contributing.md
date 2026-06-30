# Contributing to Quikk

Quikk is the Python SDK for [Chirpy](https://github.com/apalis-dev/chirpy), a Rust-based job server. This doc covers how to get a working dev environment, what's done, what's next, and a few sharp edges we've already hit so you don't have to rediscover them.

## Prerequisites

- **Python 3.12+**
- **[uv](https://docs.astral.sh/uv/)** — used for all dependency management and running tests (`uv sync`, `uv run ...`). Quikk does not use bare `pip`/`venv` directly.
- **Rust + Cargo** ([rustup.rs](https://rustup.rs)) — needed to run the Chirpy server itself for integration tests. Quikk's own code is pure Python and doesn't need Rust; only running the *server* you're testing against does.
- **`sqlite3` CLI** — handy for inspecting the Chirpy database directly (`sudo apt install sqlite3` on Debian/Ubuntu). Not strictly required, but useful for debugging storage-layer issues like the one described below.

## Setting up a local dev environment

### 1. Run the Chirpy server

You need a real Chirpy instance to run the integration tests (anything in `tests/test_integration.py` and `tests/test_protocol_integration.py`).

```bash
git clone https://github.com/apalis-dev/chirpy.git
cd chirpy
cp config.example.toml chirpy.toml
```

**Known issue — read before you run `cargo run`:** as of this writing, Chirpy's SQLite storage backend never runs its schema migrations on startup (`server.rs` calls `SqlitePool::connect_lazy()` but never `SqliteStorage::setup()`). If you use the default `:memory:` storage URL, the very first `PUSH` will panic the connection with `no such table: Jobs`. If you don't see an existing GitHub issue for this in `apalis-dev/chirpy`, please file one rather than just working around it silently — this affects any client, not just Quikk.

Both `chirpy.toml` and `chirpy.db` (created in the workaround below) are local, machine-specific files and should be gitignored in the Chirpy repo — they aren't currently, as of writing. If your `git status` shows them as untracked/modified after following these steps, add this to the Chirpy repo's `.gitignore` (a small, separate fix worth its own tiny PR or issue, not bundled into Quikk-only changes):

```
chirpy.toml
chirpy.db
chirpy.db-*
```

**Workaround until the migration bug is fixed upstream:**

```bash
cargo install sqlx-cli --no-default-features --features sqlite,rustls

# Switch chirpy.toml's [storage.sqlite] url to a real file instead of :memory:
#   url = "sqlite://chirpy.db"

touch chirpy.db
sqlx migrate run \
  --source ~/.cargo/registry/src/index.crates.io-*/apalis-sqlite-*/migrations \
  --database-url sqlite://chirpy.db
```

Then `cargo run` from the Chirpy repo root. It should print `RESP server listening on port 9101` and stay running in the foreground.

Note: a file-backed DB persists across server restarts, unlike `:memory:`. If your tests start behaving oddly because of accumulated state from previous runs, `rm chirpy.db && touch chirpy.db` and rerun the migration step.

### 2. Install Quikk

In a second terminal:

```bash
cd sdk/quikk
uv sync --all-groups
```

Make sure `.venv/`, `__pycache__/`, `*.egg-info/`, and `.pytest_cache/` are gitignored in `sdk/quikk/.gitignore` — these get regenerated locally on every `uv sync`/test run and should never be committed. If `sdk/quikk/.gitignore` doesn't already have these, add:

```
.venv/
__pycache__/
*.pyc
.pytest_cache/
*.egg-info/
```

### 3. Run the tests

```bash
# Everything except live-server tests
uv run python -m pytest tests/ -v --ignore=tests/test_integration.py --ignore=tests/test_protocol_integration.py

# Everything, with the server running
uv run python -m pytest tests/ -v
```

### Test file inventory

| File | Needs live server? | Covers |
|---|---|---|
| `test_parser.py` | No | RESP response parsing (`+`/`-`/`$`/null) |
| `test_connection.py` | No | Connection error handling against a closed port |
| `test_buffering.py` | No | Buffered reader correctness against a fragmented loopback TCP server |
| `test_models.py` | No | `Job`/`Failure`/`JobBuilder` defaults and wire-dict serialization |
| `test_commands.py` | No | Command line serializers (`HELLO`/`PUSH`/`FETCH`/etc.) |
| `test_auth_and_greeting.py` | No | Password hashing, `HI`/`BEAT` response parsing |
| `test_integration.py` | **Yes** | Phase 1: real handshake + `INFO` against a live server |
| `test_protocol_integration.py` | **Yes** | Phase 2: real `HELLO` → `PUSH` → `INFO` round trip against a live server |

New commands or models should come with both a unit test (serializer/model correctness in isolation) and, where practical, a live-server test proving the wire format is actually accepted by Chirpy — see "Integration tests are the real verification" below.

## Project layout

```
quikk/
├── transport/       # Phase 1: raw RESP wire protocol over TCP
│   ├── connection.py    # buffered async TCP connection
│   ├── parser.py         # RESP response parsing (+/-/$/null)
│   └── types.py           # response types + exception hierarchy
└── protocol/        # Phase 2: typed mapping onto the wire protocol
    ├── models.py          # Job, Failure, JobBuilder
    ├── commands.py         # command line serializers (HELLO/PUSH/FETCH/...)
    ├── greeting.py          # HI greeting + BEAT response parsing
    └── auth.py               # SCRAM-style password hashing
```

Both subpackages export everything through `quikk/__init__.py`, so consumers always do `from quikk import X` rather than reaching into internal modules.

## Status

### Phase 1 — RESP transport layer: ✅ done
TCP connection (via `anyio`, asyncio/trio agnostic), buffered line/exact-length reads (chunked socket reads served from an internal `bytearray`, not one `receive()` call per byte), and a parser covering all three response types plus null bulk strings.

### Phase 2 — Protocol & type system: ✅ done
Pydantic models for `Job`/`Failure` mirroring `job.rs` field-for-field (including the asymmetric null handling — `jid` always serializes, several other optional fields are omitted when unset). Command serializers for `HELLO`/`PUSH`/`FETCH`/`ACK`/`FAIL`/`BEAT`/`INFO`/`FLUSH`/`END`. `HI` greeting and `BEAT` response parsing, since both arrive as JSON embedded inside a RESP SimpleString rather than a structured type. Password hashing per the documented SCRAM-style intent.

**Known limitation to flag in code review, not silently fix:** Chirpy's server-side `verify_password` (in `resp/mod.rs`) has a bug where it computes a SHA-256 hash but never reassigns the result, so it ends up comparing against the raw `password + salt` string instead of an actual digest. Quikk implements hashing correctly per spec; it will not successfully authenticate against the current server until that's patched upstream. Don't "fix" this by making the client match the broken behavior — file/track the upstream bug instead.

### Phase 3 onward — not started
See the main `README.md` roadmap: connection/handshake orchestration (producer vs. worker `HELLO` modes, reconnection with backoff), the `ApalisClient` producer, the `FETCH`/`ACK`/`FAIL` worker loop, the `@quikk.task()` decorator, and structured logging/observability.

## A few conventions worth keeping

- **Mirror the Rust source, don't guess.** Every model field, default value, and skip-serialization rule in `protocol/` was checked against the actual `job.rs`/`command.rs`/`resp/mod.rs` in the Chirpy repo, not inferred from the README. If you add a new command or field, go read the corresponding Rust struct/enum first.
- **`to_wire_dict()` over generic `exclude_none`.** Pydantic's `model_dump(exclude_none=True)` would also drop `jid`, which the server expects present-but-null. Hand-roll serialization for any model with this kind of asymmetric optional-field behavior rather than reaching for a blanket flag.
- **Compact JSON.** Command serializers use `separators=(",", ":")` to match `serde_json::to_string`'s default (no whitespace). Not strictly required for correctness (the server's parser doesn't care), but keeps wire output minimal and matches the reference implementation's output byte-for-byte where it matters for tests.
- **Integration tests are the real verification.** Unit tests confirm internal consistency; only `test_integration.py` and `test_protocol_integration.py` (run against a live server) actually prove the wire format is correct. Any new command/model addition should come with a corresponding live-server test, not just a unit test against your own serializer.
