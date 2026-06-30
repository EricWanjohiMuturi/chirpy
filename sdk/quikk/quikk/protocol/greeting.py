"""
Parsers for server -> client payloads that are themselves embedded JSON
inside a RESP SimpleString, rather than a structured RESP type of their own.

Confirmed against resp/mod.rs:
  - On connect, the server sends `+HI {"v":2}\r\n`, or with auth enabled,
    `+HI {"v":2,"s":"<salt>","i":<iterations>}\r\n`.
  - A BEAT that triggers a state change (quiet/terminate) gets back
    `+{"state":"quiet"}\r\n` instead of the usual bare `+OK\r\n`.
"""

from __future__ import annotations

import json
from typing import Optional

from pydantic import BaseModel

from ..transport.types import SimpleString


class HiGreeting(BaseModel):
    v: int
    s: Optional[str] = None  # salt, present only when require_auth is set
    i: Optional[int] = None  # iterations, present only when require_auth is set


class BeatResponse(BaseModel):
    state: str


def parse_hi(message: SimpleString) -> HiGreeting:
    """Parse the initial `+HI {...}` greeting into a HiGreeting model."""
    text = message.text
    if not text.startswith("HI "):
        raise ValueError(f"Not a HI greeting: {text!r}")
    payload = json.loads(text[len("HI "):])
    return HiGreeting(**payload)


def parse_beat_response(message: SimpleString) -> Optional[BeatResponse]:
    """
    Returns a BeatResponse if the server signaled a worker state change
    (quiet/terminate), or None for a plain `+OK` heartbeat ack.
    """
    if message.text == "OK":
        return None
    payload = json.loads(message.text)
    return BeatResponse(**payload)
