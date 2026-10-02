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

## Eye point
- Rays start at the caster's eyes: the model's Head attachment, times unit scale, in the current
  pose (mounted: the rider's head on the mount; sitting, kneeling and dead have their own heights).
- The bake writes an eye-height table per display ID and stand state for the server.

## Player casts: witness ray
- The client searches for a clear segment from its eye point to any point inside the target's body
  volume, against the same baked geometry rules. It sends that one segment with the cast.
- The server accepts it when:
  1. the start is within a small tolerance of the caster's eye point;
  2. the end is inside the target's body volume, at the server's current positions with a latency
     tolerance;
  3. the segment crosses no baked triangle.
- No witness ray (or a blocked one) fails with "not in line of sight". There is no fallback search.
- This is a protocol change: casts carry the witness segment.

## Creatures (no client)
- Creature aggro, creature-vs-creature checks and creature casts use a server search: from the
  creature's eye point to the target's eyes, chest and feet. LOS holds if any ray is clear.
- Runs only when an aggro or cast check needs it. Tick cost is measured as for creature aggro.

## Bake and raycast (part 1, `crates/los`)
- `los-bake` (feature `bake`) writes `<out>/<map>/<row>_<col>.los`, `<out>/<map>/report.txt` and
  `<out>/eye_heights.tsv` from game-engine `data/`. The report lists per tile the triangle counts,
  file size, loaded memory, mean time per 30-yd ray, and every doodad model dropped as thin.
- Thickness is the smallest side of the doodad's opaque-geometry bounding box at its placed scale.
  WMO doodad sets follow the same rule. Default cutoff 1 yd (`--cutoff`).
- Each triangle is stored once, in the tile holding its centroid. A query tests the tiles under the
  segment and one ring around them. A segment touching a surface at either end is not blocked.
- Global-WMO maps (dungeons) are not baked yet.
- Open question: the Head attachment (id 20) sits on a static root bone in most models (HD human
  male: 2.149 yd in every stand state). Only 22 of 17,994 tabled displays change eye height with
  stand state. The Helm attachment (id 11) follows the head bone (human male: stand 1.85, sit 0.79,
  sleep 0.14 yd). The table uses Head as decided until the user chooses.
