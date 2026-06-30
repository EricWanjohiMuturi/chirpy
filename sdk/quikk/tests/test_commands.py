import json

from quikk import (
    Job,
    hello_line,
    push_line,
    fetch_line,
    ack_line,
    fail_line,
    beat_line,
    INFO_LINE,
    FLUSH_LINE,
    END_LINE,
)


def _parse(line: str, verb: str) -> dict:
    assert line.startswith(verb)
    rest = line[len(verb):].strip()
    return json.loads(rest)


def test_hello_line_producer_minimal():
    line = hello_line(version=2)
    assert line == 'HELLO {"v":2}'


def test_hello_line_worker_includes_all_fields():
    line = hello_line(version=2, hostname="localhost", wid="worker1", pid=1234, labels=["py"])
    data = _parse(line, "HELLO")
    assert data == {
        "v": 2,
        "hostname": "localhost",
        "wid": "worker1",
        "pid": 1234,
        "labels": ["py"],
    }


def test_push_line_contains_null_jid():
    job = Job(jobtype="test", args=["arg1"])
    line = push_line(job)
    data = _parse(line, "PUSH")
    assert data["jid"] is None
    assert data["jobtype"] == "test"
    assert data["args"] == ["arg1"]


def test_fetch_line_empty_queues():
    assert fetch_line() == "FETCH"
    assert fetch_line([]) == "FETCH"


def test_fetch_line_with_queues():
    line = fetch_line(["queue1", "queue2"])
    data = _parse(line, "FETCH")
    assert data == ["queue1", "queue2"]


def test_ack_line():
    assert ack_line("123") == 'ACK {"jid":"123"}'


def test_fail_line():
    line = fail_line("123", "RuntimeError", "failed", ["line1"])
    data = _parse(line, "FAIL")
    assert data == {
        "jid": "123",
        "errtype": "RuntimeError",
        "message": "failed",
        "backtrace": ["line1"],
    }


def test_beat_line_minimal():
    assert beat_line("worker1") == 'BEAT {"wid":"worker1"}'


def test_beat_line_with_state():
    line = beat_line("worker1", rss_kb=1024)
    data = _parse(line, "BEAT")
    assert data == {"wid": "worker1", "rss_kb": 1024}


def test_simple_command_literals():
    assert INFO_LINE == "INFO"
    assert FLUSH_LINE == "FLUSH"
    assert END_LINE == "END"
