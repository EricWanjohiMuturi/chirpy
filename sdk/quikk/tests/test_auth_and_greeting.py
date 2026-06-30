import pytest

from quikk import hash_password, parse_hi, parse_beat_response, SimpleString


def test_hash_password_deterministic():
    h1 = hash_password("secret", "salt123", 5)
    h2 = hash_password("secret", "salt123", 5)
    assert h1 == h2
    assert len(h1) == 64  # hex-encoded SHA-256 digest


def test_hash_password_changes_with_iterations():
    h1 = hash_password("secret", "salt123", 1)
    h5 = hash_password("secret", "salt123", 5)
    assert h1 != h5


def test_hash_password_rejects_zero_iterations():
    with pytest.raises(ValueError):
        hash_password("secret", "salt123", 0)


def test_parse_hi_without_auth():
    greeting = parse_hi(SimpleString('HI {"v":2}'))
    assert greeting.v == 2
    assert greeting.s is None
    assert greeting.i is None


def test_parse_hi_with_auth():
    greeting = parse_hi(SimpleString('HI {"v":2,"s":"abc123","i":1735}'))
    assert greeting.v == 2
    assert greeting.s == "abc123"
    assert greeting.i == 1735


def test_parse_beat_response_plain_ok():
    assert parse_beat_response(SimpleString("OK")) is None


def test_parse_beat_response_state_change():
    resp = parse_beat_response(SimpleString('{"state":"quiet"}'))
    assert resp is not None
    assert resp.state == "quiet"
