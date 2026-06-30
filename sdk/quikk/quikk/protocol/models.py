"""
Pydantic models mirroring the Chirpy wire protocol's job shape, defined in
the server's `src/job.rs`.

Wire-format notes (verified against job.rs, not guessed):
  - `jid` has no `skip_serializing_if` on the Rust struct, so it is always
    present on the wire, even as `null`.
  - `at`, `created_at`, `custom`, `enqueued_at`, and `failure` are all
    `#[serde(skip_serializing_if = "Option::is_none")]` — omitted entirely
    when unset, not sent as `null`.
  - Defaults (from job.rs's `default_*` functions): queue="default",
    reserve_for=1800, retry=25. `backtrace` has no default fn, so it's the
    Rust zero value: 0.
"""

from __future__ import annotations

from typing import Any, Optional

from pydantic import BaseModel, Field


class Failure(BaseModel):
    failed_at: str
    retry_count: int
    err_type: str
    message: str
    backtrace: list[str] = Field(default_factory=list)


class Job(BaseModel):
    jid: Optional[str] = None
    jobtype: str
    args: list[Any] = Field(default_factory=list)
    queue: str = "default"
    reserve_for: int = 1800
    at: Optional[str] = None
    retry: int = 25
    backtrace: int = 0
    created_at: Optional[str] = None
    custom: Optional[Any] = None
    enqueued_at: Optional[str] = None
    failure: Optional[Failure] = None

    def to_wire_dict(self) -> dict:
        """
        Build the dict that will be JSON-encoded onto the wire, replicating
        job.rs's serde skip_serializing_if behavior field-by-field rather
        than relying on a single exclude_none flag (which would also drop
        `jid`, and Chirpy's server expects `jid` to be present-but-null).
        """
        data: dict = {
            "jid": self.jid,
            "jobtype": self.jobtype,
            "args": self.args,
            "queue": self.queue,
            "reserve_for": self.reserve_for,
            "retry": self.retry,
            "backtrace": self.backtrace,
        }
        if self.at is not None:
            data["at"] = self.at
        if self.created_at is not None:
            data["created_at"] = self.created_at
        if self.custom is not None:
            data["custom"] = self.custom
        if self.enqueued_at is not None:
            data["enqueued_at"] = self.enqueued_at
        if self.failure is not None:
            data["failure"] = self.failure.model_dump()
        return data


class JobBuilder:
    """
    Fluent interface for constructing a Job with the same defaults as the
    server (queue="default", reserve_for=1800, retry=25).

    Example:
        job = (
            JobBuilder("send_email")
            .args({"to": "user@example.com"})
            .queue("emails")
            .retry(5)
            .build()
        )
    """

    def __init__(self, jobtype: str):
        self._data: dict = {"jobtype": jobtype, "args": []}

    def args(self, *args: Any) -> "JobBuilder":
        self._data["args"] = list(args)
        return self

    def queue(self, queue: str) -> "JobBuilder":
        self._data["queue"] = queue
        return self

    def retry(self, count: int) -> "JobBuilder":
        self._data["retry"] = count
        return self

    def reserve_for(self, seconds: int) -> "JobBuilder":
        self._data["reserve_for"] = seconds
        return self

    def at(self, iso_timestamp: str) -> "JobBuilder":
        self._data["at"] = iso_timestamp
        return self

    def custom(self, value: Any) -> "JobBuilder":
        self._data["custom"] = value
        return self

    def build(self) -> Job:
        return Job(**self._data)
