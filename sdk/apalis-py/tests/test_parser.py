import pytest
from transport.parser import parse_response
from transport.types import SimpleString, Error, BulkString, NullBulkString, ProtocolParseError

def test_simple_string():
    resp = parse_response(b'+OK\r\n')
    assert isinstance(resp, SimpleString)
    assert resp.text == 'OK'

def test_error():
    resp = parse_response(b'-ERR something failed\r\n')
    assert isinstance(resp, Error)
    assert resp.message == 'ERR something failed'

def test_bulk_string():
    resp = parse_response(b'$5\r\nhello\r\n')
    assert isinstance(resp, BulkString)
    assert resp.data == b'hello'

def test_null_bulk_string():
    resp = parse_response(b'$-1\r\n')
    assert isinstance(resp, NullBulkString)

def test_unknown_discriminator():
    with pytest.raises(ProtocolParseError):
        parse_response(b'*1\r\n')

def test_invalid_bulk_length():
    with pytest.raises(ProtocolParseError):
        parse_response(b'$abc\r\nhello\r\n')

def test_truncated_bulk_payload():
    with pytest.raises(ProtocolParseError):
        parse_response(b'$5\r\nhel\r\n')

def test_missing_crlf_after_bulk():
    with pytest.raises(ProtocolParseError):
        parse_response(b'$5\r\nhelloX')
