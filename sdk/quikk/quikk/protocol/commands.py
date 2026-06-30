"""
Command serializers: Python call -> Chirpy RESP wire-format line.

Mirrors `Command::to_line()` in the server's `src/resp/command.rs` exactly,
including which optional fields are sent as `null` vs. omitted. JSON is
encoded compactly (no extra whitespace) to match `serde_json::to_string`'s
default output, though the server's parser doesn't actually care about
whitespace since it uses `serde_json::from_str`.
"""

from __future__ import annotations

import json
from typing import Optional

from .models import Job

_COMPACT = {"separators": (",", ":")}


def hello_line(
    version: int = 2,
    pwdhash: Optional[str] = None,
    hostname: Optional[str] = None,
    wid: Optional[str] = None,
    pid: Optional[int] = None,
    labels: Optional[list[str]] = None,
) -> str:
    """
    Producers send only `version` (a minimal HELLO). Workers/consumers must
    additionally set hostname, wid, pid, and labels — the server uses the
    presence of all four together to decide it's registering a consumer
    (see resp/mod.rs's `if let (Some(hostname), Some(wid), Some(pid),
    Some(labels))` branch). Partial worker fields will silently be treated
    as a producer-only identification.
    """
    data: dict = {"v": version}
    if pwdhash is not None:
        data["pwdhash"] = pwdhash
    if hostname is not None:
        data["hostname"] = hostname
    if wid is not None:
        data["wid"] = wid
    if pid is not None:
        data["pid"] = pid
    if labels is not None:
        data["labels"] = labels
    return f"HELLO {json.dumps(data, **_COMPACT)}"


def push_line(job: Job) -> str:
    return f"PUSH {json.dumps(job.to_wire_dict(), **_COMPACT)}"


def fetch_line(queues: Optional[list[str]] = None) -> str:
    """An empty/omitted queue list means 'fetch from the default queue'."""
    if not queues:
        return "FETCH"
    return f"FETCH {json.dumps(queues, **_COMPACT)}"


def ack_line(jid: str) -> str:
    return f"ACK {json.dumps({'jid': jid}, **_COMPACT)}"


def fail_line(jid: str, errtype: str, message: str, backtrace: Optional[list[str]] = None) -> str:
    data = {
        "jid": jid,
        "errtype": errtype,
        "message": message,
        "backtrace": backtrace or [],
    }
    return f"FAIL {json.dumps(data, **_COMPACT)}"


def beat_line(
    wid: str,
    current_state: Optional[str] = None,
    rss_kb: Optional[int] = None,
) -> str:
    data: dict = {"wid": wid}
    if current_state is not None:
        data["current_state"] = current_state
    if rss_kb is not None:
        data["rss_kb"] = rss_kb
    return f"BEAT {json.dumps(data, **_COMPACT)}"


INFO_LINE = "INFO"
FLUSH_LINE = "FLUSH"
END_LINE = "END"
