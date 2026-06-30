"""
Password hashing for authenticated Chirpy connections.

Per the project roadmap: sha256(password + salt), iterated `n` times.

NOTE FOR MAINTAINERS: as of this writing, the server's `verify_password`
in `src/resp/mod.rs` has a bug — it loops calling `hasher.finalize()` but
never reassigns the result back into `hash`, so it ends up comparing the
client's `pwdhash` against the raw, unhashed `password + salt` string
rather than an actual SHA-256 digest. This client implements the hashing
correctly per the documented intent; it will NOT authenticate successfully
against the current server code until that bug is fixed server-side. Flag
this as a separate issue against the Chirpy repo rather than working around
it here — the wire format should mean what the roadmap says it means.
"""

from __future__ import annotations

import hashlib


def hash_password(password: str, salt: str, iterations: int) -> str:
    """
    Chained SHA-256: digest = sha256(...sha256(sha256(password + salt))...)
    applied `iterations` times. Returns the final digest as a hex string.
    """
    if iterations < 1:
        raise ValueError("iterations must be >= 1")
    digest = (password + salt).encode("utf-8")
    for _ in range(iterations):
        digest = hashlib.sha256(digest).digest()
    return digest.hex()
