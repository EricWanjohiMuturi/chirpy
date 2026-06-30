import pytest
import anyio
from quikk import Connection
from quikk import TransportError

@pytest.mark.anyio
async def test_connection_lifecycle():
    conn = Connection()
    # This should fail unless a server is running, so just check error handling
    with pytest.raises(TransportError):
        await conn.connect('localhost', 9999)
    # Not connected, so send_raw should fail
    with pytest.raises(TransportError):
        await conn.send_raw('PING')
    # Not connected, so read_response should fail
    with pytest.raises(TransportError):
        await conn.read_response()
