# Native visual-triggered send proof

## Scope

Parent2026-10-24 approves a generic fix on `triggered-native-send`, based exactly on master1fb0230adaced22b170796e37271ea4e45a6ebfb. Publish the same CastSuccess/SpellGo as an ordinary cast at triggered launch, before missile impact. Latest parent approval includes non-triggered, channel, AuraPointsOnClient, hit-delay, AlwaysCastLog and unconditional nonzero native SpellXSpellVisual terms; strictly-conditional-only definitions with no other term are an explicit logged-once/no-publication gap. Do not build or extend a condition framework. No spell-ID exceptions. Keep every existing conflicting assertion unchanged; parent owns its decision and a separate full gate. Run the server/server-simulation check once.

BM remains on separate `dps-parity-bm-howl-root` doc tipa7de7171c/runtime73a541f82; ownerb9 remains frozen. Original Boar CastSuccess expectation is preserved.

## Falsification and source

TC a352b1fa `Spell.cpp:3574–3575` routes FULL_MASK/CAST_DIRECTLY into `cast(true)`;3893 calls SendSpellGo.4779 skips only when IsNeedSendToClient is false;8348 makes a resolved nonzero visual sufficient. `SpellInfo.cpp:4525–4554` selects a native SpellXSpellVisual record after caster conditions; Spell.cpp567 assigns it. DONT_REPORT_CAST_ERROR suppresses failed-cast reports, not successful sends.

Frozen native Boar471936 has unconditional row454344→visual152700/probability1. Our launch_triggered_spell and missiles::launch omit both CastSuccess and SpellGo. Replacing the original test with Damage/Miss would hide that generic gap; rejected and reverted before commit.

Source receipt/hashes: engine `data/diagnostics/dps-parity-fix-2026-10-21/triggered-send-source-receipt.json`. Zero-visual control is actual228597 Frostbolt damage, not a modified visual row. Native public-API fixtures cover Boar impact plus success/wire publication, zero-visual impact without publication, and native116 missile launch-before-impact/exactly-once publication. 1429bef59 genuine native2FAIL/1PASS13.21s (compile16m41s): real Boar impact precedes CastSuccess0vs1 RED, native116 lacks launch publication, actual zero-visual228597 impact/no-pair controlPASS. No compiler/data failure is counted as RED.

## Representation boundary

The old `has_spell_visual` means any Difficulty0 row with nonzero child SpellVisualID; it remains unchanged for existing cosmetic consumers. New native metadata separately proves an unconditional nonzero SpellXSpellVisual RECORD ID and records conditional alternatives. No conditional is evaluated or assumed matched. `SpellInfo.cpp4525–4554` guarantees nonzero resolution whenever an unconditional nonzero row exists, regardless of the conditional winner.

Pass1 native census:2,127 strictly-conditional-only definitions. Boar471936 is not in that set;1459/2061/1953 are.1459/2061 independently send through native AuraPointsOnClient;1953 remains unresolved when triggered.487 definitions have independent channel/points/hit-delay/AlwaysCastLog reasons, leaving1,640 catalog gaps (4 missing Misc rows explicitly included). Source: `triggered-visual-pass1-with-always-cast-log.json` under the same diagnostics root. Original five-term1,641 receipt is retained, not overwritten. AlwaysCastLog removes native196415 from that gap; SharedDefines.h726 and Spell.cpp8353 are authoritative.

Runtime shares one publisher across ordinary, scripted, costed and bool FULL_MASK call boundaries, immediately before flight. It preserves explicit targets, generates no cast-success proc event for FULL_MASK, and replaces the old unconditional creature log rather than duplicating it. Logged-once unresolved state is keyed by spell only. GREEN pending; eight targeted cases now include both APIs, actual missile ordering, independent points/AlwaysCastLog terms and a real strictly-conditional Mind Blast payload without publication.

## Existing assertions: do not edit

Confirmed native-visual conflicts, unchanged:

- `scripted_ai/tests/azure_absolute_zero.rs:421–425`: `azure_telash_natural_absolute_zero_hit_launches_source_outbound_jump` expects child387981 completed SpellGo count0 with a full-mask rationale. Native unconditional record375800→visual123194 makes TC's send predicate true.
- `spell_cast/attack_restriction_tests.rs:58`: `native_attack_restriction264_rejects_native_attack_predicates` first triggers Phalanx287353, then expects the uncleared complete SpellGo inbox empty. Native unconditional record265038→visual83000 requires Phalanx's own send, independently of the correctly refused attack.
- Same file111: `native_attack_restriction264_gained_mid_steady_shot_rejects_completion` likewise triggers native visual Phalanx before the whole-inbox empty assertion. Do not weaken either test here.

Read-only inventory still in progress for other explicit no-success assumptions, including split spell-tests. Aggregate boss/corpse-silence counts are not automatically classified as conflicting without an actual triggered source.14861/Burning Cleave is not yet located in this fixed source/data baseline; no assumption that the supplied example is wrong.

## Open

- [x] Native targeted2RED/zero-control1PASS at1429; no compiler/data prerequisites mislabeled gameplay RED.
- [ ] Generic native-visual send implementation and immediate/missile/zero-control GREEN.
- [ ] Exact conflicting-test inventory/source justification sent parent, without edits.
- [ ] One two-crate check, independent proof and separate parent full gate.
