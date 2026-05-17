#RESP protocol parser for apalis-py transport layer.

from .types import SimpleString, Error, BulkString, NullBulkString, ProtocolParseError
from typing import Union

def parse_response(line: bytes) -> Union[SimpleString, Error, BulkString, NullBulkString]:
    """
    Parse a RESP response from the server.
    """
    if not line:
        raise ProtocolParseError("Empty response")
    first = line[:1]
    rest = line[1:]
    if first == b'+':
        return SimpleString(rest.decode(errors='replace').rstrip('\r\n'))
    elif first == b'-':
        return Error(rest.decode(errors='replace').rstrip('\r\n'))
    elif first == b'$':
        # Bulk string: $<len>\r\n<data>\r\n
        # Find the first CRLF
        try:
            header, payload = rest.split(b'\r\n', 1)
        except ValueError:
            raise ProtocolParseError("Malformed bulk string header")
        try:
            length = int(header)
        except ValueError:
            raise ProtocolParseError("Invalid bulk string length")
        if length == -1:
            return NullBulkString()
        if len(payload) < length + 2:
            raise ProtocolParseError("Truncated bulk string payload")
        data = payload[:length]
        if payload[length:length+2] != b'\r\n':
            raise ProtocolParseError("Missing CRLF after bulk string payload")
        return BulkString(data)
    else:
        raise ProtocolParseError(f"Unknown RESP discriminator: {first}")
