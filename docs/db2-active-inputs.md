# Private active DB2 inputs

`DB2_CSV_ROOT` selects an isolated root for `scripts/db2.py` and importer tests that use its defaults. Without this setting, the source remains the sibling engine's `data/db2`. `--csv-root` overrides the environment for the DB2 command.

`active/receipt.json` supplies the build at each command startup unless `--build` is explicit. A private root must contain both the build directory and its `active` link; no canonical link needs changing. The server runtime reads imported SQLite catalogs, plus active-source JSON catalogs such as renown; changing CSV links does not reimport a running server's SQLite data.

```text
DB2_CSV_ROOT=/private/data/db2 python3 scripts/db2.py check
DB2_CSV_ROOT=/private/data/db2 python3 scripts/db2.py import --db /private/world.db
```

The achievement-goggle process fixture also reads this active receipt and requires both imported `item` and `item_sparse` build receipts to match it exactly. Mixed/stale imports and missing source goggle rows remain errors; seven real item projections are unchanged. Regression: `python3 -m unittest scripts.tests.test_achievement_goggle_fixture`.

Tests: `python3 -m unittest scripts.tests.test_db2_active` exercises separate import processes before and after a private active-link switch, concrete spell-name replacement, build receipts, non-DB2 preservation, and refusal to read canonical data when private inputs are absent.
