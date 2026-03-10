# Real Corpus Fixture Place

This directory is reserved for anonymized real RPG source fixtures.

Rules:

1. Never commit confidential source directly.
2. Remove customer names, account IDs, emails, hostnames, API keys.
3. Replace business identifiers with placeholders:
   - `CUST001` -> `CUST_XXXX`
   - `ORDER2026` -> `ORDER_XXXX`
4. Keep syntax shape and operation patterns intact for compatibility testing.
5. Keep file extension as `.rpg` or `.txt`.

Preferred workflow:

1. Keep original corpus outside the repository.
2. Set `RPG_REAL_CORPUS_DIR` to that path during local CI.
3. Use snapshots to detect translator regressions.

