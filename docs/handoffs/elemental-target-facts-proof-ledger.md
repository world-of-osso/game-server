# Elemental source/target-fact qualification

Current consumer delegates target candidates/auras/counts/refresh using the pending shared861b655a3 contract (`with_targets`, `Bindings.dot_refresh`); parent now identifies dc525f641/ta1068 as the queued target tip. Duplicate own readers were removed; replacement awaits confirmed landing/API reconciliation/requalification. No WIP API merged.

## Split-B staging

`elemental-adapter-split-b` branches from preserved `elemental-parity-shuja@b4a269fd97f6ffad5783ed2d8f38edd1520e9fe3`. Parent-authorized merge of only confirmed masterd675bf8fb is ace6fa9d0: retain all seven added source/fixture files under `crates/server-simulation/src/dungeon_simulator/dps_parity/`, and preserve both dummy-reference and Elemental module registrations. Existing `server::simc_apl` imports and simulation server-module re-exports need no additional rewrite. Native consumer tests now belong to `-p server-simulation --lib`.

ASC `elemental-asc-overload@39214162f` remains frozen for separate lead gateea1085; no ASC changes imported here. No check run: landed d675 lacks the explicitly referenced `DotRefresh`, `native_dot_spell` and `NativeFacts::with_targets` API. Parent permits at most one -j2 check under normal admission; retain that allowance until required APIs land rather than manufacture compiler-only failures. Equipmentis1061, targetta1068 and precombat14766 remain unlanded. Migration is source-layout staging, not full-consumer/native qualification.

## Historical component proof

Historical own implementation at735a60ac632c9b28b099f1a505ea4867196b1eda:3/3PASS15.92s, no compiler warnings. This proves source/fact components, not the full Elemental fixture driver or DPS parity.

| Boundary | Observable proof |
|---|---|
| Pinned source | Unchanged82 action rows parse; unknown inactive action/option/target operand rejected |
| Caster-owned Flame Shock | Foreign-caster aura does not count; min selector chooses that native target and real188196 damage hits it, not the own-dot target |
| Caster-owned Lightning Rod | Own target count1; foreign-caster197209 ignored by min-remains selection |
| Real aura time | Fresh native maximum, non-refreshable→refreshable while still live, natural expiry→zero count/remains; no expiry/clock mutation |
| Unknowns | Invalid target, dot/debuff property and inactive unknown talent fail explicitly |

Authority: SimC76f8118 shaman.cpp2240–2254 uses caster-scoped Flame Shock/Lightning Rod target data; dot.cpp458–477 snapshots the next application for refreshability; action.cpp4630–4637 uses30% of that fresh duration. Native identities188389/197209 and the existing native maximum-duration path supply actual state.

Initial component run6683:0/3, then170b diagnostic confirmed full-source unsupported `>?`, real LB OnGlobalCooldown at0.2 with known188196/resolved188196, and FS absent before its18.2 expiry. Dummy evade cleanup is consistent with that early removal; expiry itself was not the failure. The corrected fixture waits for native GCD, permits only ordinary transient cast refusals, and keeps dummy combat with real LB hits (asserted CombatStatus), never by disabling its script or changing timers.

Command: own library ELF through `scripts/dps_parity_native.py`, filter `dps_parity::elemental_rotation:: --include-ignored --nocapture --test-threads=1`. Explicit immutable private snapshot from the qualified own-master baseline; no canonical writes. ELF SHA256 `89dc5ce973af666a4ad44d952f083bcd02de26004658bd6d5108ed750223e162`; before/after735+shared2abc identical.

Evidence: `/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/elemental-parity-262-2026-10-23/minmax-green-735a60ac6/` and preceding `target-facts-6683a0ad8/`, `minmax-red-170b82d9c/`. Native controls stay opt-in: `inputs::install_reference_catalogs` requires `DPS_PARITY_WORLD_DB`; no default-database fallback. Parent owns final integration gates. Shared lifecycle/equipment and full-native ST/3T integration remain open.
