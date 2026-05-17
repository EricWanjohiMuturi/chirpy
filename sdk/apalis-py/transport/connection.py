#Async TCP connection wrapper for apalis-py transport layer.

import anyio
from .parser import parse_response
from .types import TransportError

CRLF = b'\r\n'

class Connection:
    def __init__(self):
        self.stream = None

    async def connect(self, host: str, port: int):
        try:
            self.stream = await anyio.connect_tcp(host, port)
        except Exception as e:
            raise TransportError(f"Failed to connect: {e}")

    async def close(self):
        if self.stream is not None:
            await self.stream.aclose()
            self.stream = None

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

    async def _receive_exactly(self, n: int) -> bytes:
        buf = bytearray()
        while len(buf) < n:
            chunk = await self.stream.receive(n - len(buf))
            if not chunk:
                raise TransportError("Connection closed while reading payload")
            buf.extend(chunk)
        return bytes(buf)

    async def _read_line(self) -> bytes:
        """Read bytes until CRLF, return line without CRLF."""
        buf = bytearray()
        while True:
            byte = await self.stream.receive(1)
            if not byte:
                raise TransportError("Connection closed while reading line")
            buf += byte
            if buf[-2:] == CRLF:
                return bytes(buf[:-2])
