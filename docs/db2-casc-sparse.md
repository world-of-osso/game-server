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

Both paths are required: missing environment variables fail explicitly. Raw DB2s
are not tracked; the [reference README](../scripts/reference/forever-1.60.1.70205/README.md)
owns cache layout and provenance. Decoded JSON and schema text remain in the repo.

Set `ITEM_SPARSE_SOURCE` to the original absolute DB2 path to additionally
assert all 17 concrete records against the original multi-section file.
Assertions cover real names, levels, inventory types, quality, signed class/race
masks and stat arrays, flags, sockets, and float damage variance. Synthetic
fixtures cover variable strings followed by fields, copies/relationships,
missing/encrypted requested IDs, full-table encryption failure, and row bounds.

This proves selected ItemSparse readability, not full item or starter-kit closure.
