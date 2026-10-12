# Toy effects14 — active whole-goal coverage

Verified:2026-10-11. Goal remains all1166 authentic toy effects. Baseline masterd62e0b1:136 server-preflight candidates;843 missing runtime items,157 unsupported graphs,29 unresolved item metadata,1 unresolved on-use. Census is not rendered-effect certification.

## Batch1: source-resolved item selection

Toy membership was excluded from ordinary Retail item selection. Additive preparation and content rebuilding now retain all local Toy members with Item+ItemSparse rows, preserving every authored item restriction and existing content rows. No unsupported spell/script is admitted by this change. Baseline156 missing-item toys already have supported spell graphs; remaining graphs still fail their real mechanic preflight.

Actual source-bound fixture:64646 Bones of Transformation→91760, native creature41979/display32473,120000ms aura; cancellation restores original model. Item source/rebuild fixtures include109183 (World Shrinker, engineering2501/rank1 restriction retained),113542 (Whispers of Rai'Vosh,level10 retained), and unresolved72231 (not synthesized).

RED: actual importer misses843 source-resolved items; actual physical64646 cannot learn; copied census remains136 and reports missing content_item64646. GREEN and live proof pending. Evidence at engine `data/diagnostics/toy-effects14-2026-10-11/`.

## Remaining mechanic incidence

Whole catalog overlapping blockers: aura4=273,summon28=243,dummy3=156,script77=61,transmitted-object50=52,periodic226=35,area-trigger179=25,puppet control3=24,vehicle control4=16,transform56=14. Missing item rows must not hide these counts. Aura233 is accepted server state but authentic viewer-local payload resolution remains unimplemented; enum and aura replication are insufficient proof.

Source gaps retained: procedural cloak Type17 payload, Scoots ModifierTree303980, zero-entry transforms, remote-control/vehicle scripts, custom interaction/narrative/teleport destinations. Full per-toy identities/reasons are in the census receipts; gaps are not no-op implementations.

## Import command

From the merged server checkout, on an approved copy only:

```text
python3 scripts/add_toy_summon_templates.py --db data/world.db.new
```

No canonical database writes, UDP5000, full suite, or committed protocol override.
