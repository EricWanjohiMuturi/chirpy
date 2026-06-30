"""
Quikk — the official Python SDK for Chirpy, a language-agnostic
standalone job server built on apalis.

Public API:
    from quikk import Connection                      # Phase 1: transport
    from quikk import Job, JobBuilder, push_line       # Phase 2: protocol
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
from .protocol import (
    Job,
    Failure,
    JobBuilder,
    hello_line,
    push_line,
    fetch_line,
    ack_line,
    fail_line,
    beat_line,
    INFO_LINE,
    FLUSH_LINE,
    END_LINE,
    HiGreeting,
    BeatResponse,
    parse_hi,
    parse_beat_response,
    hash_password,
)

__version__ = "0.1.0"

__all__ = [
    # Transport (Phase 1)
    "Connection",
    "parse_response",
    "SimpleString",
    "Error",
    "BulkString",
    "NullBulkString",
    "TransportError",
    "ProtocolParseError",
    "ServerResponseError",
    # Protocol (Phase 2)
    "Job",
    "Failure",
    "JobBuilder",
    "hello_line",
    "push_line",
    "fetch_line",
    "ack_line",
    "fail_line",
    "beat_line",
    "INFO_LINE",
    "FLUSH_LINE",
    "END_LINE",
    "HiGreeting",
    "BeatResponse",
    "parse_hi",
    "parse_beat_response",
    "hash_password",
]
