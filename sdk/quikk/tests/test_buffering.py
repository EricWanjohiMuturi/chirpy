"""
Exercises the buffered reader against a real TCP socket (loopback), with the
server deliberately writing in small, awkward fragments that split lines and
bulk payloads across multiple `receive()` calls. This is the scenario the
old byte-at-a-time reader handled by accident (one receive per byte) and the
new chunked buffer must handle deliberately.
"""

import anyio
import pytest

from quikk import Connection, SimpleString, Error, BulkString, NullBulkString


async def _drip_feed(stream, data: bytes, fragment_size: int = 3):
    """Write `data` to the stream a few bytes at a time."""
    for i in range(0, len(data), fragment_size):
        await stream.send(data[i:i + fragment_size])
        await anyio.sleep(0)  # yield control, forcing separate TCP writes


@pytest.mark.anyio
async def test_read_response_across_fragmented_chunks():
    listener = await anyio.create_tcp_listener(local_host="127.0.0.1")
    host, port = listener.extra(anyio.abc.SocketAttribute.local_address)[:2]

    results = []

    async def handler(stream):
        async with stream:
            # Three responses back to back, split into tiny fragments,
            # including a bulk string whose payload spans multiple writes.
            payload = (
                b"+OK\r\n"
                b"-ERR boom\r\n"
                b"$11\r\nhello world\r\n"
                b"$-1\r\n"
            )
            await _drip_feed(stream, payload, fragment_size=3)

    async with anyio.create_task_group() as tg:
        tg.start_soon(listener.serve, handler)
        await anyio.sleep(0.05)  # let the listener come up

        conn = Connection(chunk_size=4)  # tiny chunk size to stress the buffer
        await conn.connect(str(host), port)

        results.append(await conn.read_response())
        results.append(await conn.read_response())
        results.append(await conn.read_response())
        results.append(await conn.read_response())

        await conn.close()
        tg.cancel_scope.cancel()

    assert isinstance(results[0], SimpleString) and results[0].text == "OK"
    assert isinstance(results[1], Error) and results[1].message == "ERR boom"
    assert isinstance(results[2], BulkString) and results[2].data == b"hello world"
    assert isinstance(results[3], NullBulkString)
