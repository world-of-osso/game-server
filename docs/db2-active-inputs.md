# Private active DB2 inputs

`DB2_CSV_ROOT` selects an isolated root for `scripts/db2.py` and importer tests that use its defaults. Without this setting, the source remains the sibling engine's `data/db2`. `--csv-root` overrides the environment for the DB2 command.

`active/receipt.json` supplies the build at each command startup unless `--build` is explicit. A private root must contain both the build directory and its `active` link; no canonical link needs changing. The server runtime reads imported SQLite catalogs, plus active-source JSON catalogs such as renown; changing CSV links does not reimport a running server's SQLite data.

```text
DB2_CSV_ROOT=/private/data/db2 python3 scripts/db2.py check
DB2_CSV_ROOT=/private/data/db2 python3 scripts/db2.py import --db /private/world.db
```

Tests: `python3 -m unittest scripts.tests.test_db2_active` exercises separate import processes before and after a private active-link switch, concrete spell-name replacement, build receipts, non-DB2 preservation, and refusal to read canonical data when private inputs are absent.

Achievement catalogs use `python3 scripts/import_achievements.py --db2-dir <engine>/data/db2/active --db <private-world.db>`. The importer validates the installed build, active receipt FDIDs/layouts, CSV SHA256s and row counts before its transaction. Columns are selected by name, so CriteriaTree Flags and OrderIndex cannot be interchanged by raw decoder field offsets. Output records the active build key and table receipts, including encrypted-record exclusions. Frozen historical decoder fixtures keep their explicit source directory; live imports never fall back to them.
