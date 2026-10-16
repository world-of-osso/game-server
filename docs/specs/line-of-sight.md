# Line of sight

Decided with the user on 2026-10-02. This replaces TrinityCore's single eye-to-eye vmap ray with
a witness-ray scheme over baked visible geometry. The deviation from Retail is deliberate:
Retail's LOS errors ("can see it, not in line of sight") are the bugs to avoid.

## Blocking geometry (baked, never read from assets at runtime)
- A bake tool reads ADT/WDT/WMO/M2 from the local CASC extraction once per client build (and on deploy)
  and writes one LOS file per map tile: triangles plus a prebuilt BVH. Like TrinityCore's vmap step,
  but keeping the visible opaque geometry instead of collision hulls.
- Kept: terrain; WMO walls/floors/ceilings (opaque render batches); large solid M2 doodads.
- Dropped:
  - alpha-tested batches (grates, leaves, fences);
  - transparent/blended batches (glass, water, effects);
  - small ground clutter;
  - doodads thinner than a cutoff measured from the model's own bounding box (start near 1 yd; tuned
    on real models, with the dropped set listed in the bake report).
  Thin objects practically never hide a whole body from every point, so ignoring them is accepted. The
  remaining risk (a pole fully hiding a target) is rare and harmless.
- Runtime: the server loads baked tile files per active tile, like ground data; no M2 or texture
  access. Measure memory per loaded tile and time per ray.

## M2 ground companion
- `los-bake` also writes `los/<map>/m2/<row>_<col>.los` from dedicated M2 bounding collision arrays, including placed active WMO doodad sets. `--m2-only` rebakes just this companion, without altering visible LOS geometry or navigation.
- GroundMap and native client movement use the same companion with shared walkable-normal, step-up and height-search rules. Floor faces are admitted by authored collision normals rotated into world space (upward within the shared slope limit), without LOS thickness or material exclusions; they are not substituted for the visible LOS bake.
- Legacy global-WMO WDTs resolve MODF name indices through the authored MWMO path and local community listfile; FileDataID-flagged MODF entries retain direct identity. No asset rewriting or network lookup.
- A non-global WDT without MAID is terrain-less only when its complete MAIN has no active tiles; active or malformed MAIN remains an explicit gap.
- Ground support contract: [M2 floor collision](../../../game-engine/docs/specs/m2-floor-collision.md).

## Eye point
- Rays start at the caster's eyes, from the model's own eye geometry: the centroid of the eye
  vertices (the eyeglow geoset, group 17), skinned by their bones in the current pose (stand, sit,
  kneel, sleep, dead), times unit scale. Models with no eye geometry use their head bone's position
  in the current pose; models with neither use the top centre of the model's bounding box (MD20
  `bounding_box`) in every state, and the bake fails when even that is missing. Mounted units use the rider's eyes on the mount. User decision 2026-10-02:
  "the character models literally have eyes"; the Head/Helm attachment heights are not used.
- The bake writes this eye point per display ID and stand state for the server
  (`eye_heights.tsv`: model space, the display's scales applied, `source` eyes, head or bounds), posed
  at the first frame of the state's animation. A model without the state's sequence (e.g. the
  human models have no Dead) holds its Stand eye. `SIT_CHAIR` (stand state 2) holds the Stand eye too: no
  Retail animation is established for it (uncited; TrinityCore, AzerothCore, WMVx, wow_client,
  WoWee, WebWowViewerCpp and solarityclient map it to none; chairs seat units in states 4-6). User
  decision 2026-10-02. `player_displays.tsv` maps race and sex to the
  character display (`ChrRaceXChrModel` → `ChrModel.DisplayID`).

## Player casts: authoritative target visibility
- At cast processing, the server searches from its authoritative caster eye point to the target's
  authoritative eyes, chest and feet, using the same baked and live-hull geometry as creature casts.
  Any clear ray permits LOS; no clear ray fails with `CastFailReason::LineOfSight`.
- Client-sent `SpellCastIntent.witness` endpoints do not authorize or reject target LOS. Moving more
  than 2 yd while a request is in transit must not invalidate an otherwise clear cast. Missing,
  displaced or stale client segments neither bypass a server wall nor block a clear server ray.
- This replaces the previous chosen endpoint tolerances; it is not a tolerance increase or fallback.
  The existing wire field remains readable but is not retained in the internal cast queue or validation.
- Check at player cast start for unit-targeted spells without `SPELL_ATTR2_IGNORE_LINE_OF_SIGHT`;
  do not recheck player LOS at cast-time completion. Server-started player casts use the same rule.
- The native client samples local range and cast origin from the same current local player pose;
  its client-side ray search does not substitute for authoritative validation.

## Area hits
- Source-area, destination-area, conditioned area and living raid-area unit selections
  require LOS from each recipient to the selected area center, after the authored target cap.
  An area hit does not reuse a caster-to-target witness or substitute the caster for a destination.
- `SPELL_ATTR2_IGNORE_LINE_OF_SIGHT` bypasses this check. Spell Attr5 AlwaysAoeLineOfSight
  or effect AlwaysAoeLineOfSight additionally requires caster-to-recipient visibility.
- Reference: TrinityCore `a352b1fa` `Spell::SelectImplicitAreaTargets` passes `center` to
  `AddUnitTarget`, then `CheckEffectTarget` checks `IsWithinLOS(target, *losPosition)`.
  `WorldObject::IsWithinLOS(point)` raises the destination by the recipient's collision height;
  this project's eye-geometry contract uses the recipient's baked eye height instead.
- Native Arcane Explosion1449 wall/visible-recipient damage regression and destination/flag
  selection controls cover this boundary. Chain, cone and persistent-area-aura LOS are not
  certified by those tests.

## Auto-attacks
- Player and creature auto-attack swings require LOS before they land, using the same
  server eye-to-eyes/chest/feet search as creature casts (no client witness).
- A blocked ready swing deals no damage or combat/proc events and does not reset either
  hand's timer or stop attacking. It resumes as soon as sight returns.
- Players receive `CastFailReason::LineOfSight` for Attack (6603), displayed as
  "Target not in line of sight" (`SPELL_FAILED_LINE_OF_SIGHT`), through the existing
  spell error channel. Creature failures are not sent to the victim.
- Without the LOS resource, checks behave exactly as existing spell LOS checks.

## Creatures (no client)
- Creature aggro, creature-vs-creature checks and creature casts use a server search: from the
  creature's eye point to the target's eyes, chest and feet. LOS holds if any ray is clear.
- Runs only when an aggro or cast check needs it. Tick cost is measured as for creature aggro.
- Chest is halfway between the target's feet and eyes. The eye point is turned to the unit's
  facing. Aggro rays run last, after every other `CanStartAttack` test, nearest candidate
  first, stopping at the first one seen.
- Creature casts at a unit target fail with "not in line of sight" unless the spell has
  `SPELL_ATTR2_IGNORE_LINE_OF_SIGHT` (TrinityCore `Spell::CheckCast`), at cast start and finish.
- The server loads the bake from `<ground dir>/los` (`GAME_SERVER_GROUND_DIR`, else
  `game-engine/data`): the eye table at startup (it refuses to start without it), each map's
  tiles on first use. A map without baked tiles has nothing that blocks. A unit whose display has no eye
  row (a skipped template display that a script spawns) fails closed: it is out of everyone's line of sight, and
  the server logs it once per display, naming the first unit (creature entry and entity) that
  showed it. Not modelled yet: mounted eyes, unit scale
  (`OBJECT_FIELD_SCALE`).

## Bake and raycast (`crates/los`, runtime in `shared::los`)
- The runtime (tile files, segment queries, eye table, `Body`) is shared-protocol `shared::los`,
  which the server and the Godot client link; `crates/los` is the bake and re-exports it.
- The eye table must cover every display a spawned creature shows (its template's models and spawn
  display overrides) and every `ChrModel` character display: the bake fails when any of them lacks
  a row in any stand state. The other `content_creature_template_model` displays (summons,
  transforms) bake when they can; those that cannot are skipped and listed in `report.txt` with
  why (no `CreatureDisplayInfo` row, a model without geometry, a sequence whose `.anim` the model
  does not name). User decision 2026-10-02. `scripts/los-extract-models.py --world-db data/world.db
  --engine ../game-engine` first extracts from local CASC whatever of those models, primary skins,
  skeletons and stand-state `.anim` files is missing.
- `los-bake` (feature `bake`) writes `<out>/<map>/<row>_<col>.los`, `<out>/<map>/report.txt` and
  `<out>/eye_heights.tsv` from game-engine `data/`. The report lists per tile the triangle counts,
  file size, loaded memory, mean time per 30-yd ray, and every doodad model dropped as thin.
- Thickness is the smallest side of the doodad's opaque-geometry bounding box at its placed scale.
  WMO doodad sets follow the same rule. Default cutoff 1 yd (`--cutoff`).
- Each triangle is stored once, in the tile holding its centroid. A query tests the tiles under the
  segment and one ring around them. A segment touching a surface at either end is not blocked.
- The shared collector supports global-WMO placement. `los-bake`'s CLI remains continent-only;
  [navigation's bake tool](../wiki/systems/navigation.md) also writes Stockade (map34) LOS tiles
  from that collector for the existing LOS-query acceptance oracle. Ship this supplement with
  the existing eye tables; it is not a second extractor.
- Run: `cargo run --release -p los --features bake --bin los-bake -- --data ../game-engine/data
  --world-db data/world.db --out ../game-engine/data/los --map 0 --all` (per map; `--no-eyes`
  skips the eye table). Map 0 is 1176 tiles, 6.3 GB, under 10 minutes (debug, opt-level 2). `deploy.sh` ships the
  whole `los/` directory beside the ground files and stops when the eye table is missing.
- Resolved: the Head attachment (id 20) sits on a static root bone in most models, so the eye point
  comes from the eye geometry instead (see Eye point).
