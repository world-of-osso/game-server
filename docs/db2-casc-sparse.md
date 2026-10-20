# Bounded sparse WDC5 decoding

`scripts/db2_casc.py::decode` supports observed Sparse + Index flags `0x5`
without changing fixed-record decoding. Source: DBCD
`DBCD.IO/Readers/WDC5Reader.cs` section ordering and sequential field reads;
matching official `ItemSparse.dbd` layout `6FCC3191`, build `1.60.1.70205`.

Supported: uncompressed 8/16/32-bit numeric elements, schema-declared arrays,
inline UTF-8 NUL strings, copy rows, and index-based relationship maps.
Widths come from signed field metadata (`32 - Bits`, otherwise compression
width); strings advance the cursor by actual byte length. Observed rows may
have up to three zero bytes aligning their end to four bytes.
Other sparse flags/compressions/widths fail explicitly.

## API

- `text_columns`: stored field index plus one for the external ID, **before**
  array flattening, as in existing fixed-record decoding.
- `inline_arrays`: stored field index to element count, including narrow arrays.
- `field_types`: stored field index to `INTEGER` (signed metadata width), `REAL`
  (float32), or `UNSIGNED` (default). Sparse-only; obtain declarations from the
  matching DBD, not numeric guesses. Returned arrays flatten into output columns.
- `record_ids`: every requested ID must resolve from a readable row or copy;
  zero-filled encrypted sections never produce placeholder rows. Without this
  selection, sparse decoding rejects encrypted sections. Fixed-record decoding
  instead reports and skips zero-filled encrypted sections and reads already
  decrypted sections, preserving the existing behavior.

## Fixed-record decoding

`read_fixed(data, id_field=None)` is the one fixed-record WDC5 parser.
It returns the header, field infos, and per section either the records (ID, per-field values, packed bits, parent, record offset, global index) plus copy pairs, or `records=None` for a zero-filled encrypted section.
`decode()` builds its rows from it, and so do the engine's `export_db2_csv.read_wdc5` and `import_npc_appearance.read_wdc5`.
`read_string` / `read_string_at` resolve string offsets through the virtual concatenation of all section string blocks.
A string that lands in a zero-filled encrypted section fails rather than reading as empty.

Scalar pallets precede array pallets in the pallet block, each in field order.

Fixed latent bug (2026-10-20):

- **Symptom.** `decode()` read pallets in plain field order. Real PlayerCondition `1D743D99` (FDID 1045411, build 70391) declares array pallet field 48 MovementFlags before scalar pallets 49 WeaponSubclassMask and 56 ContentTuningID. In field order:
  - every one of its 48,491 rows read WeaponSubclassMask 1048866 and ContentTuningID 958 instead of 0;
  - 21 rows had shifted MovementFlags;
  - the deduplicated ContentTuningID pallet repeated six entries.
- **Impact.** No server import was affected: `world.db` PlayerCondition comes from engine-exported CSVs, which already read scalar pallets first.
- **Regression.** The test needs `PLAYER_CONDITION_DB2` (SHA-256 pinned in the test):

```sh
PLAYER_CONDITION_DB2=/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/wow-formats-python-readers-2026-10-20/db2/1045411.db2 \
  python3 -m unittest scripts.tests.test_db2_casc_player_condition
```

## Concrete evidence and targeted tests

The untracked engine cache `data/db2/fixtures/item_sparse_6fcc3191_selected.db2`
retains exact bytes of the
17 selected clear records plus original field metadata from FDID `1572924`.
Only header counts, section offsets, and offset/ID lists were reduced/relocated;
no record values were changed. Original SHA-256:
`15fde0ce086da0e642011b1f99b49977b6dfa56e72cf4edc941401dddf139f6d`.
Source: engine `data/forever-1.60.1.70205/starter-kit-probe/1572924.db2`;
original offsets/sizes: adjacent `item-sparse-selected-readability.json`.
Schema: engine `data/forever-1.60.1.70205/definitions/ItemSparse.dbd`.

Run from server checkout (no extraction, database import, or services):

```sh
export FOREVER_DB2_SOURCE=/home/osso/Projects/world-of-osso/game-engine/data/db2/forever-1.60.1.70205
export ITEM_SPARSE_SELECTED_FIXTURE=/home/osso/Projects/world-of-osso/game-engine/data/db2/fixtures/item_sparse_6fcc3191_selected.db2
python3 -m unittest scripts.tests.test_db2_casc_sparse scripts.tests.test_skyborne_data.SkyborneDecodeTest
```

Tests that need an unset variable skip and name it (`scripts/tests/fixture_env.py`); set both to run them. Raw DB2s
are not tracked; the [reference README](../scripts/reference/forever-1.60.1.70205/README.md)
owns cache layout and provenance. Decoded JSON and schema text remain in the repo.

Set `ITEM_SPARSE_SOURCE` to the original absolute DB2 path to additionally
assert all 17 concrete records against the original multi-section file.
Assertions cover real names, levels, inventory types, quality, signed class/race
masks and stat arrays, flags, sockets, and float damage variance. Synthetic
fixtures cover variable strings followed by fields, copies/relationships,
missing/encrypted requested IDs, full-table encryption failure, and row bounds.

This proves selected ItemSparse readability, not full item or starter-kit closure.
