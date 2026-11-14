#!/usr/bin/env python3
"""Print the base58 sha256 of a wasm file (= NEAR global contract code hash)."""
import hashlib
import sys

from pubkey import b58e

print(b58e(hashlib.sha256(open(sys.argv[1], "rb").read()).digest()))
