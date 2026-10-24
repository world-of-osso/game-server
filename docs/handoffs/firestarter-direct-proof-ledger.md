# Direct Firestarter — proof ledger

Status: qualified-current targeted4GREEN on7ae36007a. Lead full-suite gate fs1058 and PlayerST/3T recheck remain separate; no DPS parity claim.

## Scope

133/11366 direct critical chance only. TC a352b1fa `spell_mage.cpp762–779` reads actual caster205026 effect0 and sets100% when recipient HP percentage is greater than or equal to that amount. Missing aura/effect or lower HP leaves ordinary chance intact. Native70448 amount90 is data, not a new hardcoded threshold. No periodic321712 executor/hook, kit/data changes, grants, coefficients or stat changes.

## Mechanism RED

- Revision1e1c15d2d297c225a7f7214ec9381b596858b676, job01a16aad-6785-7582-8f35-7613a35f9890.
- Actual selected native90 Player/book/aura/traits/gear; ordinary casts, selected205026e0=90.133 at100% HP noncritical19752 and11366 noncritical33485.89% control observes both ordinary critical/noncritical outcomes.
-1PASS/2assertionFAIL24.18s, compile27m09, exit101. Test filter `firestarter_native90_ -- --ignored --nocapture --test-threads=1` through build-host/agent-run rotation-b, normalcapacityADD, explicit-j2, sim/serveropt1.
- Evidence: engine `data/diagnostics/rotation-fidelity-2026-10-21/firestarter-1e1c-{hp-native.log,mixed-input-diagnostic-receipt.json}`. Old private catalogs mixed with live startup DB: genuine conditional-mechanism failures, NOT qualified current-input/DPS evidence.

## Qualified current-input GREEN

Revision7ae36007af15686096243f50bbc0fbd7e1a6c176 includes required pd1035/master2333d924b. Job01a16b51-6ff4-7412-a8de-3d08ce9cc4ff:4PASS/0FAIL27.90s, compile19m57, exit0, zero compiler warnings.133/11366 at100/native90% HP are critical;89% preserves both ordinary outcomes. Exact startup inode assertion also passes.

Fresh CURRENT canonical SQLite backup00:39:04Z2026-10-24: canonicalSHA256568200017bcb3b5ed6f17931fc0994fff502c5c801f93f96197fc718bcd493bf stable/noWAL, codeHEAD46c2bf608f81a5fa585bd63b32b2db6346e843da; copySHAdd1de231e32c26291182dff050a8de5a5132908723ed21454eb128e45e4b05ae. Maintained `scripts/dps_parity_native.py` Cargo runner binds startup paths to that private DB; actual childTMPDIR=/var/tmp/osso. Authorized lead_fire_14779 admission, one-j2job, no bypass.

Evidence in engine `data/diagnostics/rotation-fidelity-2026-10-21/`: `firestarter-7ae-lead-native.log`, `world-firestarter-current-20261024T003904Z.receipt.json`, `firestarter-direct-7ae-gate-packet.json`. Both changed runtime source and native test file hash-match compiled native root682b0eeeea6ae5f1bbea-45f25f5c0625. Earlier2c2b baseline and normal7ae attempts endedNO_ADMISSION, not tests.

Lead gate fs1058 integrates crate-split master8e7c7a685. Current-canonical PlayerST/3T uses the same compiled7ae binary/new snapshot. First recheck refused before combat because DPS_PARITY_ASSISTED_DB2 was missing, not a gameplay failure. Corrected runner uses the frozen70448 AssistedCombat CSVs with recorded hashes; measurement remains pending. No source/kit/data/stat changes or DPS claim.

## Contract and tests

- [Aura contract](../specs/auras.md#firestarter-direct-critical-strikes).
- `crates/server/src/dungeon_simulator/dps_parity/firestarter_tests.rs`:100/native90 HP guarantees and native89% ordinary critical/noncritical control.
- Implementation: existing `spell_cast::scripts::crit_chance` optional override; ordinary native damage/critical executor remains unchanged.
