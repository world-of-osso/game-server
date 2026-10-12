# Toy effects14 — active whole-goal coverage

Verified:2026-10-11. Goal remains all1166 authentic toy effects. Baseline masterd62e0b1:136 server-preflight candidates;843 missing runtime items,157 unsupported graphs,29 unresolved item metadata,1 unresolved on-use. Census is not rendered-effect certification.

## Batch1: source-resolved item selection

Toy membership was excluded from ordinary Retail item selection. Additive preparation and content rebuilding now retain all local Toy members with Item+ItemSparse rows, preserving every authored item restriction and existing content rows. No unsupported spell/script is admitted by this change. Baseline156 missing-item toys already have supported spell graphs; remaining graphs still fail their real mechanic preflight.

Actual source-bound fixture:64646 Bones of Transformation→91760, native creature41979/display32473,120000ms aura; cancellation restores original model. Item source/rebuild fixtures include109183 (World Shrinker, engineering2501/rank1 restriction retained),113542 (Whispers of Rai'Vosh,level10 retained), and unresolved72231 (not synthesized).

RED: actual importer misses843 source-resolved items; actual physical64646 cannot learn; copied census remains136 and reports missing content_item64646. GREEN: importer/source equality/idempotency/rebuild/rollback4/4; real physical learn/UseToy/display/cancel test and copied census2/2. Prepared census136→292 (+156,zero removals); canonical unchanged136. Live proof pending. Evidence at engine `data/diagnostics/toy-effects14-2026-10-11/`.

## Batch2: source-defined zero-entry costumes

Pinned local TC a352b1fa `SpellAuraEffects.cpp:2090-2242` defines Murloc42365→21723, Gnomeregan75531→male31654/female31655, Darkspear75532→male31737/female31738. Retail69933 SpellEffect confirms transform56/misc0 for all3. Native sex determines Pride appearance; existing transform ownership/restoration machinery remains authoritative. All five display identities exist in local CreatureDisplayInfo. No inferred random-costume table, race mapping, model substitution or zero-entry no-op. RED actual33079 UseToy refused aura56. GREEN actual learning/UseToy/source models/cancellation for all3 bothsexes. Murloc's real6000ms cast is awaited; Pride casts are instant. Prepared census292→295. Both-skin private5911 native proof uses Gnomeregan Pride54651/display31654, plus item-boundary Bones64646/display32473, and restores native models after each cancellation. Ten1280×720 PNGs at `/syncthing/AgentShared/2026-10-11/toy-effects14/`; four transformed frames visually inspected. Authenticated local-CASC frozen-source closure204 assets published117 missing receipts/47 aliases; no alternate model or cache substitution.

## Batch3: highest-count aura4, complete ordinary-model caster kits

Of273 remaining aura4 toy identities,96 roots have the unchanged complete-zero dummy-only shape and a visual binding.27 have ordinary native model/animation/sound kits without procedural, positioner, conditional or missing-model payloads. Source descriptions still distinguish gameplay/script contracts: this batch admits only12 caster-only visual actions. Magi225826/Unstable225828 area recipients are deferred to their own behavioral proof, not silently treated as caster auras. Source data is Retail69933; actual local TDB1210 `spell_script_names` scans4066 bindings with0 selected bindings. Absence alone is not proof; exact descriptions + complete kit chains establish the selected cosmetic contracts.

|Item|Spell|Authored action / duration ms|
|---|---|---|
|108743|160688|Smoldering boot attachment /600000|
|110586|160093|Held mysterious flowers /180000|
|116115|170869|Fiery wing attachments /900000|
|127864|188172|Overhead spotlight /3600000|
|137294|212660|Kirin Tor marker, Mage class mask128 retained /300000|
|147537|242014|Demon Hunter Pepe head attachment,2000ms cast /3600000|
|163744|280053|Light snowfall art, not gameplay weather /300000|
|180947|335276|Back-mounted Tithe vessel /900000|
|182890|340276|Quill/parchment and dictation animation /180000|
|204220|403491|Authored Hraxian manifestation art /300000|
|230727|467285|Explosive Victory aura art /1800000|
|259240|1264653|Held wine glass and toast animation /300000|

All12 retain zero base/misc/trigger/item/mechanic fields and caster targeting1. The full aura/visual/event/kit/attachment/effect-name records are `aura4-selected-source.json`; all other96-shape candidate records retained in `aura4-excluded-source.json`. Types used2/5/6; native runtime already implements them. No source renderer type1/procedural17, GenericID, Positioner or conditional viewer choice is claimed. RED actual108743 UseToy refuses aura4. GREEN1 exercises all12 real bag learn/UseToy/SpellGo/exact lifetimes/cancel, unchanged health/global model and no actors;182.74s. Re-census295→307,zero removals. Private5911 actual Blazing Wings116115/170869 source519369 wing model applies/removes with unchanged native player in both skins; wing PNGs visually inspected. Native closure277files all locally present,208model assets authenticated against frozen69933 source;69Oggs separately content-key/SHA verified. Audible output not certified. No generic dummy/no-op executor; not a claim all12 toys have per-toy rendered proof.

## Remaining mechanic incidence

Whole catalog overlapping blockers: aura4=273,summon28=243,dummy3=156,script77=61,transmitted-object50=52,periodic226=35,area-trigger179=25,puppet control3=24,vehicle control4=16,transform56=14. Missing item rows must not hide these counts. Aura233 is accepted server state but authentic viewer-local payload resolution remains unimplemented; enum and aura replication are insufficient proof.

Source gaps retained: procedural cloak Type17 payload, Scoots ModifierTree303980, zero-entry transforms, remote-control/vehicle scripts, custom interaction/narrative/teleport destinations. Full per-toy identities/reasons are in the census receipts; gaps are not no-op implementations.

## Import command

From the merged server checkout, on an approved copy only:

```text
python3 scripts/add_toy_summon_templates.py --db data/world.db.new
```

No canonical database writes, UDP5000, full suite, or committed protocol override.
