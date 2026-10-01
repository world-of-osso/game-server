# Crafting reference import

Import the seven genuine crafting reference tables from `data/reference/midnight-20261001` into an explicitly selected staging SQLite database. [Import mechanics and schema](../wiki/reference/crafting-reference-import.md).

## What it must do

- [x] Require `--db` and `--source-dir`; select no implicit database or source.
- [x] Import exactly seven tables, preserving unrelated data and replacing all seven in one transaction.
- [x] Pin build 12.1.0.69933 source SHA-256, DB2 layout, ordered CSV headers and exact row counts; reject invalid rows without suppression.
- [x] Preserve all sixteen CraftingData columns, including unknown names and float32 values; remove only the validated duplicated ModifiedCraftingSpellSlot relationship parent.
- [x] Report per-table source, build, hash, layout and count deterministically; support quality/item and slot/category/reagent joins.

## How it works

- [Crafting reference import](../wiki/reference/crafting-reference-import.md)

## Implementation inventory

- `scripts/crafting_data.py`: pinned decoder/import CLI and atomic replacement.
- `scripts/crafting_schema.sql`: complete ordered schema for the seven tables.
- `scripts/db2_casc.py`: existing WDC5 decoder, reused unchanged.

## Tests asserting this spec

- `scripts/tests/test_crafting_data.py`: concrete WDC5/CSV fixtures, invalid-source and transaction rollback, real-source staging joins, explicit CLI paths.

## Known gaps (current cycle)

None within the bounded reference import.

## Out of scope

- Live `world.db` import, deployment, economy catalog/content policy changes and modern recipe resolution: separately owned work.
- Additional reference tables or runtime crafting behavior: not part of this seven-table slice.
