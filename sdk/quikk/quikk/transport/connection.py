# Async TCP connection wrapper for the Quikk transport layer.

import anyio

from .parser import parse_response
from .types import TransportError

CRLF = b'\r\n'
DEFAULT_CHUNK_SIZE = 4096


class Connection:
    def __init__(self, chunk_size: int = DEFAULT_CHUNK_SIZE):
        self.stream = None
        self._chunk_size = chunk_size
        # Internal read buffer. We pull bytes from the socket in chunks and
        # serve line/exact reads from this buffer, instead of issuing one
        # `receive()` call per byte.
        self._buf = bytearray()

    async def connect(self, host: str, port: int):
        try:
            self.stream = await anyio.connect_tcp(host, port)
        except Exception as e:
            raise TransportError(f"Failed to connect: {e}")

    async def close(self):
        if self.stream is not None:
            await self.stream.aclose()
            self.stream = None
        self._buf.clear()

    async def send_raw(self, command: str):
        if self.stream is None:
            raise TransportError("Not connected")
        # Ensure exactly one CRLF
        if not command.endswith('\r\n'):
            command += '\r\n'
        await self.stream.send(command.encode())

    async def read_response(self):
        if self.stream is None:
            raise TransportError("Not connected")
        # Read until CRLF for header
        header = await self._read_line()
        if header.startswith(b'$'):
            # Bulk string: need to read payload
            try:
                length = int(header[1:])
            except ValueError:
                raise TransportError("Invalid bulk string length")
            if length == -1:
                return parse_response(header + CRLF)
            payload = await self._receive_exactly(length + 2)
            return parse_response(header + CRLF + payload)
        else:
            return parse_response(header + CRLF)

    async def _fill_buffer(self):
        """Pull one chunk from the socket into the internal buffer."""
        chunk = await self.stream.receive(self._chunk_size)
        if not chunk:
            raise TransportError("Connection closed by peer")
        self._buf.extend(chunk)

    async def _receive_exactly(self, n: int) -> bytes:
        while len(self._buf) < n:
            await self._fill_buffer()
        data = bytes(self._buf[:n])
        del self._buf[:n]
        return data

    async def _read_line(self) -> bytes:
        """Read bytes until CRLF, return line without CRLF."""
        while True:
            idx = self._buf.find(CRLF)
            if idx != -1:
                line = bytes(self._buf[:idx])
                del self._buf[:idx + len(CRLF)]
                return line
            await self._fill_buffer()
