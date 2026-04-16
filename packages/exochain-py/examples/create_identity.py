"""Create a new EXOCHAIN identity and sign a message.

Run:
    python examples/create_identity.py
"""

from __future__ import annotations

from exochain import Identity


def main() -> None:
    # Generate a fresh Ed25519 keypair with a derived DID.
    alice = Identity.generate("alice")
    print(f"DID:        {alice.did}")
    print(f"Public key: {alice.public_key_hex}")
    print(f"Label:      {alice.label}")

    # Sign a message.
    message = b"I agree to the terms."
    signature = alice.sign(message)
    print(f"Signature:  {signature.hex()}")

    # Verify — anyone with the public key can check the signature.
    ok = Identity.verify(alice.public_key_hex, message, signature)
    print(f"Verifies:   {ok}")


if __name__ == "__main__":
    main()
