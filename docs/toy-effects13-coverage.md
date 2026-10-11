# Toy-effects13 — nonzero and linked cosmetic dummy auras

Verified: 2026-10-11. Base `47c35c4` (origin/toy-effects12). Server-preflight coverage baseline **134/1,166**; final census/proof below. Native tint, shell placement and procedural material rendering are **not certified**.

## Ranked remaining handlers

Read-only local runtime census, eligible item definitions/conditions, excluding creature-summon roots (effect28) and creature-summon/control failures. Transmitted scenery and fishing replacements remain in this ranking, as in effects12. Counts overlap; full exact item/refusal rows: `data/diagnostics/toy-effects13-2026-10-11/baseline-ranking.json`.

|Handler|Blocked toys before|Decision|
|---|---:|---|
|aura4 dummy|31|Highest-yield source-closed subset first: nonzero cosmetic aura and mixed native-trigger cosmetic chain below. Remaining scripts are not blanket admitted.|
|effect3 dummy|30|No newly source-closed destination, corpse, skinning, persistent-map or appearance-selection action.|
|effect77 script|16|No sourced dice probabilities/sides, weather choices, return-ball decisions or context destinations.|
|effect50 transmitted object|11|Existing handler; specialized interactions/missing templates, not an absent universal spawner.|
|aura56 transform|8|All zero-entry selections lack an authentic creature/display choice.|

The aura4 count differs from effects12's32 because the explicit effect28-root exclusion also removes Wormhole Generator48933. No toy disappears from the underlying census.

## Newly handled subsets

- **88377 Turnip Paint /127803:** source aura4 amount1, target21 (selected friendly player), **6000ms**, visual26722. The authored chain includes orange projectile model199730 (`fruit_orange.m2`), explosion513071 (`orange_explosion.m2`), and procedural3681 Type1. Preserve amount1 and normal aura replication/expiry; no damage, healing, global model replacement or fabricated color coefficients.
- **88417 Gokk'lok's Shell /128150:** source aura4 amount1 plus native trigger64 children **128151/128152**. All three auras target the caster, last **120000ms**, and have the source movement-interrupt bit0x8. Root visual26913 includes attachment/bubble models199933/165760; children bind24256/24429, procedural Type17 records3243–3246/3292–3294. Keep all three spell identities and source amounts; ordinary trigger/application/expiry/movement cancellation runs, without gameplay shell objects, collision, teleport or summoned actors.

Admission is **per audited aura slot**, with exact source amount, no misc/trigger/item/mechanic payload, and a visual binding. Other effects/descendants must independently pass their existing runtime handlers. The original complete-zero cosmetic predicate is unchanged. This closes preflight refusal at the existing real aura/trigger/lifetime boundary, not a generic dummy executor or a rendering implementation.

Sources: local Retail12.1.0.69933 DB2 CSVs, local community-listfile, read-only world.db. Complete payload/visual/event/kit/attachment/model chains retained in `visual-source.json`; model paths in `model-paths.json`. No downloads or database writes.

## Exact retained gaps

[Effects12's identity/reason table](toy-effects12-coverage.md#exact-retained-gaps) remains the SSOT for all unchanged refusals **except88377/88417**, which are closed above. Further inspection retained:

- **37254 Simian Sphere:**48332 has real transform1108 and child48333, but48333 has no visual binding and only unscripted dummy4 (“You're going Ape!”). Its runtime behavior is not derivable from that marker; do not silently drop it.
- **43499 Iron Boot Flask:**58501 has real transform29370 and proc58505, but root dummy slot2 has no visual binding/source action. A proc/transform alone does not authorize the dummy slot.
- **115503 Lava Pendant:**171352 art is known, but the Draenor-only contract's complete region/instance gating is not supplied by the zero dummy. No map1116-only approximation.
- **133998 Rainbow Generator:**203534 art does not specify the promised holiday-dependent cooldown reduction.
- **199902 compass /138873 Frosh:** retain unsourced positioner/GenericID and procedural-effect consumers; attachment replication is not proof of orientation or full appearance.

All other remaining toy identities retain their exact runtime reason in the final census. All11 refused summon-control category identity sets must match the baseline; no control changes authorized.

## Proof ledger

All Cargo runs use `agent-run toy-effects13`. Shared protocol source is a frozen local archive of **44b2b0f**, selected with Cargo `--config` path patches for shared and lightyear_netcode. Current shared checkout `b452322` cannot compile this base server: inherited quest snapshot initialization lacks `expires_at`, `failed`, `timer_sampled_at`. No unrelated quest fix or shared checkout mutation. Current-protocol integration remains blocked by that existing drift.

- RED: base plus new tests, frozen protocol44b2b0f: **4/4 failed** on actual UseToy/preflight `unsupported toy spell: aura4`; read-only baseline census134. `red.log`, `red-census.json`.
- GREEN and final format/check/census: pending at this document's initial implementation commit.
