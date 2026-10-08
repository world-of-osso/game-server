# Forever Zephras reference capture

`scripts/wowhead_forever.py` captures per-page Wowhead Forever references for reconstructed Zephras Isle content. It does not import server content. [Content import](../wiki/systems/content-import.md#forever-zephras-reference-capture) owns source provenance and calibration evidence.

## What it must do

- [x] Preserve embedded Listviews, Gatherer/entity data, mapper observations, display IDs and quest markup with URL, timestamp and scrape-directory provenance; reject challenge pages. Cached pages retain their original timestamp.
- [x] Normalize quest text, starter/ender IDs, race/class restrictions, series, counted kill/collect/interact targets and rewards, including reward markup omitted from Listview rows. Preserve unknown credit types rather than treating every NPC objective as a kill.
- [x] Expose zone-list metadata separately from captured entity pages and enumerate missing pages. Missing displays and coordinates must not be invented.
- [x] Convert UiMap percentages to WoW X/Y consistently with the retail Goldshire map fixture; sample ADT MCVT inner vertices and respect terrain holes.
- [x] Retry transient GET failures with bounded delays; stop immediately on HTTP 403 without caching an error page.
- [ ] Capture all Zephras-listed pages sequentially, at least 1.5 seconds apart; stop on 429 and retain source for parser failures. Discovery must not follow NPCs in unrelated zones.
- [ ] Report grounded inferred starts separately from source-authored quest/race requirements; do not write starts into server code.

## How it works

- [Content import](../wiki/systems/content-import.md#forever-zephras-reference-capture)
- [Skyborne prerequisites](../wiki/systems/skyborne.md#map-2991-start-requirements)

## Implementation inventory

- `scripts/wowhead_forever.py` — sequential capture, source parsing, normalization, coordinate inversion and terrain sampling.
- `scripts/tests/fixtures/wowhead_forever/` — trimmed captured pages; no game assets.

## Tests asserting this spec

- `scripts/tests/test_wowhead_forever.py` — saved-page normalization, retail coordinate inversion, terrain fan/hole sampling, missing-page visibility, cache and HTTP failure behavior.

## Known gaps (current cycle)

- [ ] CloudFront blocks remaining entity pages; browser also receives 403.
- [ ] Captured pages do not identify a race-96 level-1 intro. Preserve original requirements pending the importing agent's decision.
- [ ] Terrain-only Z does not establish WMO floors, water safety or playable collision closure.

## Out of scope

- Server publication and gameplay acceptance belong to the later import slice.
- Scripted events, phasing, facing and asset/CDN downloads are excluded by the approved reconstruction scope.
