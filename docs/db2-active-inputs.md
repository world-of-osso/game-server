# Private active DB2 inputs

## Native Curve type publication

Verified: 2026-10-17. `scripts/db2.py import` publishes readable installed `Curve.csv` rows as `curve(ID, Type, Flags)` alongside `curve_point`, with an explicit `db2_build` receipt. Installed70448 uses pinned WoWDBDefs `d6f50d4f351193e84226aa62a36faa0f58528afa`, `Curve.dbd` layout `D362535A`: ID32, Typeu8, Flags32. Required Type/Flags values are copied from the CSV, never inferred from curve points.

The installed export contains62817 readable rows out of62819 declared records. Curve IDs **121405** and **121406** belong to unknown-key encrypted section `DAC5AA105D0C1B67`; both have `content_authenticated=false` and are omitted, never fabricated or defaulted. Their IDs come from the unencrypted WDC5 encrypted-record-ID list, not guessed zero-filled payloads. A future consumer requesting a missing Curve ID must fail explicitly rather than assume Type0, a radius or another curve. Readable33927 has Type0/Flags0; its consumer remains blocked until the typed catalog is published.

The process/schema regression `python3 -m unittest scripts.tests.test_db2_curves` exercises native8/33927 types/flags, build receipts, replacement on regeneration and retained Retail/Skyborne rows. It uses private fixtures; it does not publish canonical data. A real publication receipt must separately record the COPY-only import's exact command, input/output SHA256, row changes and both excluded IDs. No radius fallback or scale consumer is introduced by this publisher change.

## Private CSV roots

`DB2_CSV_ROOT` selects an isolated root for `scripts/db2.py` and importer tests that use its defaults. Without this setting, the source remains the sibling engine's `data/db2`. `--csv-root` overrides the environment for the DB2 command. Import also requires an explicit `--source-dir` for the separate raw Forever overlay; tests and `import-azerothcore.sh` require `FOREVER_DB2_SOURCE`. See [untracked cache paths](../scripts/reference/forever-1.60.1.70205/README.md#local-data-paths).

`active/receipt.json` supplies the build at each command startup unless `--build` is explicit. A private root must contain both the build directory and its `active` link; no canonical link needs changing. The server runtime reads imported SQLite catalogs, plus active-source JSON catalogs such as renown; changing CSV links does not reimport a running server's SQLite data.

```text
DB2_CSV_ROOT=/private/data/db2 python3 scripts/db2.py check
DB2_CSV_ROOT=/private/data/db2 python3 scripts/db2.py import --source-dir "$FOREVER_DB2_SOURCE" --db /private/world.db
```

The achievement-goggle process fixture also reads this active receipt and requires both imported `item` and `item_sparse` build receipts to match it exactly. Mixed/stale imports and missing source goggle rows remain errors; seven real item projections are unchanged. Regression: `python3 -m unittest scripts.tests.test_achievement_goggle_fixture`.

Tests: `python3 -m unittest scripts.tests.test_db2_active` exercises separate import processes before and after a private active-link switch, concrete spell-name replacement, build receipts, non-DB2 preservation, and refusal to read canonical data when private inputs are absent.

Achievement catalogs use `python3 scripts/import_achievements.py --db2-dir <engine>/data/db2/active --db <private-world.db>`. The importer validates the installed build, active receipt FDIDs/layouts, CSV SHA256s and row counts before its transaction. Columns are selected by name, so CriteriaTree Flags and OrderIndex cannot be interchanged by raw decoder field offsets. Output records the active build key and table receipts, including encrypted-record exclusions. Frozen historical decoder fixtures keep their explicit source directory; live imports never fall back to them.
