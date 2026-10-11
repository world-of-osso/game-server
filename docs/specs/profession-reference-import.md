# Profession reference import

Import the nine additional pinned Retail 12.1.0.69933 profession tables into an explicitly selected staging database. Existing [crafting references](crafting-reference-import.md) remain separate. [Source receipt and gaps](../wiki/reference/profession-reference-import.md).

## What it must do

- [ ] Validate SHA-256, layout, raw record count, decoded row count, identity, field widths and finite floats before opening a destination.
- [ ] Preserve DB2 copy rows, relationships and float32 values; no fabricated field semantics or quality/concentration formulas.
- [ ] Replace only the nine named tables atomically; preserve unrelated tables and roll back schema/rows on failure.
- [ ] Support a no-write preflight and explicit staging import without selecting canonical world.db.

## How it works

- [Profession reference import](../wiki/reference/profession-reference-import.md).

## Implementation inventory

- `scripts/profession_data.py`: exact pins, source decoding and staging import.
- `scripts/db2_casc.py`, `scripts/crafting_data.py`: existing decoder, finite-value/ID validation and build identity.

## Tests asserting this spec

- `scripts/tests/test_profession_data.py`: actual local-CASC rows, numeric/relationship preservation, repeat import, no-DB preflight, corrupt-source refusal and transaction rollback.

## Known gaps (current cycle)

- [ ] Quality thresholds, reagent contributions, concentration curve composition/rounding and item bonus selection need an authentic evaluator; data identities are not formulas.
- [ ] Source ingestion is not specialization, crafting-order or recrafting runtime acceptance.

## Out of scope

Canonical database mutation: lead runs an explicit import after review. No automatic rebuild integration or seven-table compatibility changes.
