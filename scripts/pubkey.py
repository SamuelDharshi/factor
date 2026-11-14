#!/usr/bin/env python3
"""Derive the ed25519 public key from a NEAR secret key (ed25519:<base58 64 bytes = seed||pub>)."""
import sys

A = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"


def b58d(s):
    n = 0
    for c in s:
        n = n * 58 + A.index(c)
    raw = n.to_bytes((n.bit_length() + 7) // 8, "big")
    return b"\0" * (len(s) - len(s.lstrip("1"))) + raw


def b58e(b):
    n = int.from_bytes(b, "big")
    out = ""
    while n:
        n, r = divmod(n, 58)
        out = A[r] + out
    return "1" * (len(b) - len(b.lstrip(b"\0"))) + out


if __name__ == "__main__":
    sk = sys.argv[1].split(":", 1)[-1]
    raw = b58d(sk)
    if len(raw) != 64:
        sys.exit("expected a 64-byte ed25519 secret key")
    print("ed25519:" + b58e(raw[32:]))
