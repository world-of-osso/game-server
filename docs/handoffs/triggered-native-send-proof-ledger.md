# Native visual-triggered send proof

## Scope

Parent2026-10-24 approves a generic fix on `triggered-native-send`, based exactly on master1fb0230adaced22b170796e37271ea4e45a6ebfb. Publish the same CastSuccess/SpellGo as an ordinary cast at triggered launch, before missile impact, gated only by resolved nonzero native SpellXSpellVisual. No spell-ID exceptions. Keep every existing conflicting assertion unchanged; parent owns its decision and a separate full gate. Run the server/server-simulation check once.

BM remains on separate `dps-parity-bm-howl-root` doc tipa7de7171c/runtime73a541f82; ownerb9 remains frozen. Original Boar CastSuccess expectation is preserved.

## Falsification and source

TC a352b1fa `Spell.cpp:3574–3575` routes FULL_MASK/CAST_DIRECTLY into `cast(true)`;3893 calls SendSpellGo.4779 skips only when IsNeedSendToClient is false;8348 makes a resolved nonzero visual sufficient. `SpellInfo.cpp:4525–4554` selects a native SpellXSpellVisual record after caster conditions; Spell.cpp567 assigns it. DONT_REPORT_CAST_ERROR suppresses failed-cast reports, not successful sends.

Frozen native Boar471936 has unconditional row454344→visual152700/probability1. Our launch_triggered_spell and missiles::launch omit both CastSuccess and SpellGo. Replacing the original test with Damage/Miss would hide that generic gap; rejected and reverted before commit.

Source receipt/hashes: engine `data/diagnostics/dps-parity-fix-2026-10-21/triggered-send-source-receipt.json`. Zero-visual control is actual228597 Frostbolt damage, not a modified visual row. Native public-API fixtures cover Boar impact plus success/wire publication, zero-visual impact without publication, and native116 missile launch-before-impact/exactly-once publication. New targeted RED pending; no runtime edit yet.

## Representation boundary

Current SpellInfo stores only `has_spell_visual`: any Difficulty0 row with nonzero SpellVisualID. It does not store a resolved SpellXSpellVisual record ID or evaluate caster conditions. This presence bool is not silently renamed or claimed to be full TC resolution; parent notified before implementation.

## Existing assertions: do not edit

Confirmed native-visual conflicts, unchanged:

- `scripted_ai/tests/azure_absolute_zero.rs:421–425`: `azure_telash_natural_absolute_zero_hit_launches_source_outbound_jump` expects child387981 completed SpellGo count0 with a full-mask rationale. Native unconditional record375800→visual123194 makes TC's send predicate true.
- `spell_cast/attack_restriction_tests.rs:58`: `native_attack_restriction264_rejects_native_attack_predicates` first triggers Phalanx287353, then expects the uncleared complete SpellGo inbox empty. Native unconditional record265038→visual83000 requires Phalanx's own send, independently of the correctly refused attack.
- Same file111: `native_attack_restriction264_gained_mid_steady_shot_rejects_completion` likewise triggers native visual Phalanx before the whole-inbox empty assertion. Do not weaken either test here.

Read-only inventory still in progress for other explicit no-success assumptions, including split spell-tests. Aggregate boss/corpse-silence counts are not automatically classified as conflicting without an actual triggered source.14861/Burning Cleave is not yet located in this fixed source/data baseline; no assumption that the supplied example is wrong.

## Open

- [ ] Native targeted RED/control; no compiler/data prerequisites mislabeled gameplay RED.
- [ ] Generic native-visual send implementation and immediate/missile/zero-control GREEN.
- [ ] Exact conflicting-test inventory/source justification sent parent, without edits.
- [ ] One two-crate check, independent proof and separate parent full gate.
