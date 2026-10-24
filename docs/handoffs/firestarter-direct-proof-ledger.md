# Direct Firestarter — proof ledger

Status: implementation committed for targeted GREEN; no current-input acceptance or DPS parity claim.

## Scope

133/11366 direct critical chance only. TC a352b1fa `spell_mage.cpp762–779` reads actual caster205026 effect0 and sets100% when recipient HP percentage is greater than or equal to that amount. Missing aura/effect or lower HP leaves ordinary chance intact. Native70448 amount90 is data, not a new hardcoded threshold. No periodic321712 executor/hook, kit/data changes, grants, coefficients or stat changes.

## Mechanism RED

- Revision1e1c15d2d297c225a7f7214ec9381b596858b676, job01a16aad-6785-7582-8f35-7613a35f9890.
- Actual selected native90 Player/book/aura/traits/gear; ordinary casts, selected205026e0=90.133 at100% HP noncritical19752 and11366 noncritical33485.89% control observes both ordinary critical/noncritical outcomes.
-1PASS/2assertionFAIL24.18s, compile27m09, exit101. Test filter `firestarter_native90_ -- --ignored --nocapture --test-threads=1` through build-host/agent-run rotation-b, normalcapacityADD, explicit-j2, sim/serveropt1.
- Evidence: engine `data/diagnostics/rotation-fidelity-2026-10-21/firestarter-1e1c-{hp-native.log,mixed-input-diagnostic-receipt.json}`. Old private catalogs mixed with live startup DB: genuine conditional-mechanism failures, NOT qualified current-input/DPS evidence.

## Required current-input proof

Original worktree remains frozen2c2bbfa2d83cc8fc0c66838c7904f3679f2fa80b, including required pd1035/master2333d924b. Sole current baseline job01a16ae9-8f4d-70ca-9b0f-7e25f1c203dc waits normalcapacity. Its script makes a fresh CURRENT canonical SQLite backup afterADD, records canonical/private SHA256 and codeHEAD, runs existing native source-view wrapper and startup inode assertion alongside3direct tests. Inner env restores actualTMPDIR=/var/tmp/osso after agent-run.

This implementation uses the separate existing idle worktree/branch; queued baseline inputs are untouched. No second own compile. Qualified-current baseline and GREEN receipts remain pending; independent final gate belongs to lead.

## Contract and tests

- [Aura contract](../specs/auras.md#firestarter-direct-critical-strikes).
- `crates/server/src/dungeon_simulator/dps_parity/firestarter_tests.rs`:100/native90 HP guarantees and native89% ordinary critical/noncritical control.
- Implementation: existing `spell_cast::scripts::crit_chance` optional override; ordinary native damage/critical executor remains unchanged.
