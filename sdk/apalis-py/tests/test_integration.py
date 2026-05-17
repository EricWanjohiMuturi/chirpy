import os
import pytest
import anyio
from transport.connection import Connection
from transport.types import SimpleString, BulkString, Error, NullBulkString

CHIRPY_HOST = os.getenv("CHIRPY_HOST", "localhost")
CHIRPY_PORT = int(os.getenv("CHIRPY_PORT", "9101"))
HELLO = 'HELLO {"v":2}'

@pytest.mark.anyio
async def test_greeting_and_info():
    conn = Connection()
    await conn.connect(CHIRPY_HOST, CHIRPY_PORT)

    greeting = await conn.read_response()
    assert isinstance(greeting, SimpleString)

    await conn.send_raw(HELLO)
    hello_resp = await conn.read_response()
    assert isinstance(hello_resp, SimpleString)

    await conn.send_raw("INFO")
    resp = await conn.read_response()
    assert isinstance(resp, BulkString)
    assert b"chirpy" in resp.data
    await conn.close()

@pytest.mark.anyio
async def test_invalid_command():
    conn = Connection()
    await conn.connect(CHIRPY_HOST, CHIRPY_PORT)

    await conn.read_response()
    await conn.send_raw(HELLO)
    await conn.read_response()

    await conn.send_raw("FOOBAR")
    resp = await conn.read_response()
    assert isinstance(resp, Error)
    await conn.close()