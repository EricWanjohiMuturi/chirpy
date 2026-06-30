# Response types and error classes for the Quikk transport layer.

class SimpleString:
    def __init__(self, text: str):
        self.text = text
    def __repr__(self):
        return f"SimpleString({self.text!r})"

class Error:
    def __init__(self, message: str):
        self.message = message
    def __repr__(self):
        return f"Error({self.message!r})"

class BulkString:
    def __init__(self, data: bytes):
        self.data = data
    def __repr__(self):
        return f"BulkString({self.data!r})"

class NullBulkString:
    def __repr__(self):
        return "NullBulkString()"

class TransportError(Exception):
    pass

class ProtocolParseError(Exception):
    pass

class ServerResponseError(Exception):
    pass
