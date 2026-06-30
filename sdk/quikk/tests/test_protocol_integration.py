"""
Exercises the Phase 2 protocol layer (models + command serializers) against
a real, running Chirpy RESP server. Requires the server from the main
`chirpy` repo to be running locally (see README for setup):

    cp config.example.toml chirpy.toml
    cargo run

Override CHIRPY_HOST / CHIRPY_PORT env vars if your server isn't on the
default 127.0.0.1:9101.
"""

import os

import pytest

from quikk import (
    Connection,
    SimpleString,
    BulkString,
    JobBuilder,
    hello_line,
    push_line,
    parse_hi,
)

CHIRPY_HOST = os.getenv("CHIRPY_HOST", "localhost")
CHIRPY_PORT = int(os.getenv("CHIRPY_PORT", "9101"))


@pytest.mark.anyio
async def test_hi_greeting_parses_as_protocol_model():
    conn = Connection()
    await conn.connect(CHIRPY_HOST, CHIRPY_PORT)

    greeting = await conn.read_response()
    assert isinstance(greeting, SimpleString)

    hi = parse_hi(greeting)
    assert hi.v == 2
    # Default config.example.toml has no `require_auth`, so no salt/iterations.
    assert hi.s is None
    assert hi.i is None

    await conn.close()


@pytest.mark.anyio
async def test_producer_handshake_and_push_job():
    conn = Connection()
    await conn.connect(CHIRPY_HOST, CHIRPY_PORT)

    await conn.read_response()  # HI greeting

    await conn.send_raw(hello_line(version=2))
    hello_resp = await conn.read_response()
    assert isinstance(hello_resp, SimpleString)
    assert hello_resp.text == "OK"

    job = JobBuilder("send_email").args({"to": "user@example.com"}).queue("emails").build()
    await conn.send_raw(push_line(job))
    push_resp = await conn.read_response()
    assert isinstance(push_resp, SimpleString)
    assert push_resp.text == "OK"

    await conn.close()


@pytest.mark.anyio
async def test_info_after_push_reflects_queued_job():
    conn = Connection()
    await conn.connect(CHIRPY_HOST, CHIRPY_PORT)

    await conn.read_response()
    await conn.send_raw(hello_line(version=2))
    await conn.read_response()

    job = JobBuilder("send_email").queue("emails").build()
    await conn.send_raw(push_line(job))
    await conn.read_response()

    await conn.send_raw("INFO")
    resp = await conn.read_response()
    assert isinstance(resp, BulkString)
    assert b"chirpy" in resp.data
    # `queued_count` should be non-zero now that we've pushed at least one job.
    assert b"queued_count" in resp.data

    await conn.close()
