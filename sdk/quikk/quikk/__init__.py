"""
Quikk — the official Python SDK for Chirpy, a language-agnostic
standalone job server built on apalis.

Public API (Phase 1 — transport layer):
    from quikk import Connection

Phase 2 (protocol/type system: Job, Failure, command serializers) will be
exposed here once implemented, keeping `import quikk` as the single
entrypoint for consumers.
"""

from .transport import (
    Connection,
    parse_response,
    SimpleString,
    Error,
    BulkString,
    NullBulkString,
    TransportError,
    ProtocolParseError,
    ServerResponseError,
)

__version__ = "0.1.0"

__all__ = [
    "Connection",
    "parse_response",
    "SimpleString",
    "Error",
    "BulkString",
    "NullBulkString",
    "TransportError",
    "ProtocolParseError",
    "ServerResponseError",
]
