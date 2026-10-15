# Runtime Toy casting data

Server startup requires `data/toy_required_areas.json` and `data/toy_facing_requirements.json`, alongside `data/world.db`. Missing files, invalid JSON, absent source build or area groups without members fail loudly. `spell_cast::toy_requirements::load` inserts startup-owned data into the world; casting never reads an embedded production snapshot or a process-global cache. [Contract](specs/toys.md#runtime-generated-casting-requirements).

## DB2-follow apply

Run from the current game-server checkout after importing the current canonical DB2 into `data/world.db`. `CURRENT_CSV_DIR` is the canonical export selected by that apply; `SOURCE_BUILD` is that export's verified build, not a separately guessed installed version. `TDB_DUMP` is the source world dump already used by the apply's model-geometry importer. Both generators require the build argument and preserve CSV identity hashes.

```sh
python3 scripts/generate_toy_area_requirements.py --db data/world.db --casting-requirements "$CURRENT_CSV_DIR/SpellCastingRequirements.csv" --area-group-members "$CURRENT_CSV_DIR/AreaGroupMember.csv" --build "$SOURCE_BUILD" --output data/toy_required_areas.json
python3 scripts/generate_toy_facing_requirements.py --database data/world.db --requirements "$CURRENT_CSV_DIR/SpellCastingRequirements.csv" --dump "$TDB_DUMP" --build "$SOURCE_BUILD" --output data/toy_facing_requirements.json
```

No generator exports its own DB2 or mutates world.db. Facing flags come from the selected Toy roots; target bounding radii come from the source dump, not guessed CombatReach values. [Facing/area behavior](specs/toys.md#source-area-group-closure--active2026-10-13).

## Deployment

The lead's server data sync must include both generated JSON files with world.db before starting the new binary. These files are data artifacts, not tracked server-source inputs. Regenerate them in the DB2-follow apply step; rebuilding the server cannot update their contents.

## Proof boundaries

`toydata27_*` loader tests use tiny temporary files and a new app per data version, asserting source-version changes and actual facing refusal. Toy cast tests retain a compact69-root area fixture and a single42247 radius row in test-only setup; no production radius blob is compiled into tests. The private process probe checks startup refusal when required files are absent. Existing area/facing/coin behavior and native ribbon artifact verification remain separate proofs.
