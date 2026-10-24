# Native visual-triggered send proof

## Scope

Parent2026-10-24 approves a generic fix on `triggered-native-send`, based exactly on master1fb0230adaced22b170796e37271ea4e45a6ebfb. Publish the same CastSuccess/SpellGo as an ordinary cast at triggered launch, before missile impact. Latest parent approval includes non-triggered, channel, AuraPointsOnClient, hit-delay, AlwaysCastLog and unconditional nonzero native SpellXSpellVisual terms; strictly-conditional-only definitions with no other term are an explicit logged-once/no-publication gap. Do not build or extend a condition framework. No spell-ID exceptions. Parent owns assertion decisions and a separate full gate. Latest decision approves changing only the two Phalanx inbox-empty assertions; Azure/Omor stay unchanged and run in the authoritative full gate, without local tracing. Run the server/server-simulation/server-spell-tests `--tests -j2` check once.

Parent requested exact master1466a4ca7 integration; mergeca66b03e4 retains both FULL_MASK periodic provenance and shared publication. No independent BM edits, pushes or agents. Original Boar CastSuccess expectation is preserved.

## Falsification and source

TC a352b1fa `Spell.cpp:3574–3575` routes FULL_MASK/CAST_DIRECTLY into `cast(true)`;3893 calls SendSpellGo.4779 skips only when IsNeedSendToClient is false;8348 makes a resolved nonzero visual sufficient. `SpellInfo.cpp:4525–4554` selects a native SpellXSpellVisual record after caster conditions; Spell.cpp567 assigns it. DONT_REPORT_CAST_ERROR suppresses failed-cast reports, not successful sends.

Frozen native Boar471936 has unconditional row454344→visual152700/probability1. Before this fix, launch_triggered_spell and missiles::launch omitted both CastSuccess and SpellGo. Replacing the original test with Damage/Miss would hide that generic gap; rejected and reverted before commit.

Source receipt/hashes: engine `data/diagnostics/dps-parity-fix-2026-10-21/triggered-send-source-receipt.json`. Zero-visual control is actual228597 Frostbolt damage, not a modified visual row. Native public-API fixtures cover Boar impact plus success/wire publication, zero-visual impact without publication, and native116 missile launch-before-impact/exactly-once publication. 1429bef59 genuine native2FAIL/1PASS13.21s (compile16m41s): real Boar impact precedes CastSuccess0vs1 RED, native116 lacks launch publication, actual zero-visual228597 impact/no-pair controlPASS. No compiler/data failure is counted as RED.

## Representation boundary

The old `has_spell_visual` means any Difficulty0 row with nonzero child SpellVisualID; it remains unchanged for existing cosmetic consumers. New native metadata separately proves an unconditional nonzero SpellXSpellVisual RECORD ID and records conditional alternatives. No conditional is evaluated or assumed matched. `SpellInfo.cpp4525–4554` guarantees nonzero resolution whenever an unconditional nonzero row exists, regardless of the conditional winner.

Pass1 native census:2,127 strictly-conditional-only definitions. Boar471936 is not in that set;1459/2061/1953 are.1459/2061 independently send through native AuraPointsOnClient;1953 remains unresolved when triggered.487 definitions have independent channel/points/hit-delay/AlwaysCastLog reasons, leaving1,640 catalog gaps (4 missing Misc rows explicitly included). Source: `triggered-visual-pass1-with-always-cast-log.json` under the same diagnostics root. Original five-term1,641 receipt is retained, not overwritten. AlwaysCastLog removes native196415 from that gap; SharedDefines.h726 and Spell.cpp8353 are authoritative.

Runtime shares one publisher across ordinary, scripted, costed and bool FULL_MASK call boundaries, immediately before flight. It preserves explicit targets, generates no cast-success proc event for FULL_MASK, and replaces the old unconditional creature log rather than duplicating it. Logged-once unresolved state is keyed by spell only. Native8GREEN18.28s at83ef9b551 and8GREEN12.41s after master integration atca66b03e4 cover both APIs, actual missile ordering, independent points/AlwaysCastLog terms and a real strictly-conditional Mind Blast payload without publication. All six approved predicate terms and logged-once storage were also read against pinned TC; no isolated channel/hit-delay-only or log-capture proof is claimed.

## Existing assertions and parent decisions

Confirmed native-visual conflicts, initially run unchanged:

- `scripted_ai/tests/azure_absolute_zero.rs:421–425`: `azure_telash_natural_absolute_zero_hit_launches_source_outbound_jump` expects child387981 completed SpellGo count0 with a full-mask rationale. Native unconditional record375800→visual123194 makes TC's send predicate true.
- `spell_cast/attack_restriction_tests.rs:58`: `native_attack_restriction264_rejects_native_attack_predicates` first triggers Phalanx287353, then expects the uncleared complete SpellGo inbox empty. Native unconditional record265038→visual83000 requires Phalanx's own send, independently of the correctly refused attack.
- Same file111: `native_attack_restriction264_gained_mid_steady_shot_rejects_completion` likewise triggers native visual Phalanx before the whole-inbox empty assertion.

Atca66b03e4 the unchanged server subset is11PASS/4FAIL35.32s: both Phalanx assertions genuinely fail; Azure and Omor stop earlier at `data_bosses.rs312` with Python launch EACCES, not gameplay RED. Gate exports and post-agent-run `TMPDIR=/var/tmp/osso` were used; interpreter-only probes inside the same scope/mounts resolve and execute `/usr/bin/python3`. Parent assigns Azure/Omor unchanged to its authoritative full gate and explicitly forbids tracing. The premerge follower `min_active_enemies` setup failure is resolved by master integration; its original control passes now.

Parent then approves only the two Phalanx assertion changes: allow Phalanx SpellGo in those inboxes, continue rejecting every other SpellGo, retain other empty-inbox and refusal/damage checks. Each changed assertion cites TC Spell.cpp8346–8353. No CastSuccess-empty assertion existed at those sites, so none is added or relaxed. The original failures remain in saved logs; changed assertions await the parent full gate.

Read-only inventory across server/simulation/split spell-tests found no additional explicit successful-trigger silence assertion. Six controls with blanket-silence comments were included unchanged; Omrogg, periodic dummy, shell, wormhole and follower motion pass, Omor is setup-blocked as above. Split refusal/dead-caster execution was admission-waiting when the parent narrowed the final step to one check; no split result is claimed. Aggregate corpse/early-cast silence checks are not misclassified as triggered-send conflicts.14861/Burning Cleave is absent from this fixed native baseline.

## Recovery proof artifacts

Engine `data/diagnostics/dps-parity-fix-2026-10-21/` holds `triggered-merged-proof.pyrun.py`, `triggered-merged-compile.log` (PASS28m33s, zero diagnostics), `triggered-merged-native8.log`, `triggered-merged-existing-server.log`, executable receipts and command/revision/world hashes in `proof-ledger.jsonl`. World snapshot SHA256 remains `0297adfd466e85ce812f7821aa3788823adcf06867a9bd6999207bad3a75d83a`. Shared protocol is clean at2abc7fa844. Native8 evidence covers unchanged production code after the assertion-only edit; it does not certify the parent full gate.

The single requested `cargo check --locked -p server -p server-simulation -p server-spell-tests --tests -j2` passes at75dfa0e8b in162.88s (2m42s, zero warnings). Admission and `agent-run triggered-spellgo env TMPDIR=/var/tmp/osso` are recorded in `triggered-final-check-result.json` and `triggered-final-check-75dfa0e8b.log`. Earlier admission-only continuations were stopped before any check began; this command ran once. Subsequent edits only record this proof. The changed Phalanx assertions have compile proof, not a claimed new runtime GREEN; the parent's full gate owns execution.

## Open

- [x] Native targeted2RED/zero-control1PASS at1429; no compiler/data prerequisites mislabeled gameplay RED.
- [x] Generic whole-predicate implementation and eight native immediate/missile/zero-control cases GREEN atca66b03e4.
- [x] Conflicts/source justification sent parent; only the two Phalanx assertions changed after explicit approval.
- [x] One three-package `--tests -j2` check PASS at75dfa0e8b; committed evidence accompanies the tip handoff.
- [ ] Parent full gate, including unchanged Azure/Omor and changed Phalanx assertions.
