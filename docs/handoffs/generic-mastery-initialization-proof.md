# Generic mastery initialization qualification — 2026-10-24

## Scope and root cause

Preserve the aura318-only mastery base introduced by0576893d7, fix actual follower initialization, and qualify the19 newly failing sa1089 cases without changing existing behavioral assertions. No push or deployment. The tip retains e4126a743 (native item damage-use/fractional charge work) and merges qualification baseline1fb0230ad in dd3f387ce.

Native `SkillLineAbility`25920 learns114585 from skill183, AcquireMethod2/MinSkillLineRank1/SpellLevel0. `skill_race_class_info` row5 admits all classes/races at level1. Ordinary player progression already learned this passive. Followers use source-neutral race0, which rejected that explicit `[-1,-1]` race mask, dropping114585. The previous unconditional8 in `compute_character_stats` concealed the omission. e587f83a7 admits explicit universal masks for race0, retains restricted-mask rejection, and gates runtime mastery on knowing a mastery spell of the active specialization.

TC a352b1fa `Player.cpp30545–30550` (`CanUseMastery`) checks the active specialization's mastery spell IDs. `StatSystem.cpp549–568` gates aura318 plus rating, not a spec-derived or hardcoded base. Native114585 supplies the base through its actual aura. See [character stats](../wiki/systems/character-stats.md#initialization-and-knowledge-gate--2026-10-24).

## Production RED before the fix

At5d1c30c81, before the production fix or master merge:

`agent-run mastery-init cargo test -p server --lib native_mastery_ -j2 -- --nocapture`

Result:4PASS/3FAIL,14.10s after17m13s compilation. `/home/osso/.worktrees/logs/mastery-init-production-red.log`.

- Ordinary character created with the real persistence/create kit, progressed from1 to90, then passed its learned book through real passive application/reapplication: **PASS**,114585 present and8 raw mastery.
- Actual initialized Garrick follower with StatsPlugin: **FAIL** at `known.contains(114585)`, before the raw-point assertion.
- Existing AP and BM summon regressions: **FAIL** at positive-mastery assertions. Existing base lifecycle, timed refresh and ordinary-NPC exclusion controls passed.

## Qualification proof ledger

| Revision / scope | Command and result | Later invalidation |
|---|---|---|
| d96bfa968; includes1fb0230ad | `agent-run mastery-init cargo test -p server -p server-spell-tests --lib -j2 -- --nocapture` plus exact filters in `/home/osso/.worktrees/logs/mastery-init-qualification-filters.txt`: server33PASS/2FAIL,41.65s; compile30m28s. Log `mastery-init-qualification.log`. | AP and the newly added follower test were corrected afterward; other passing cases unchanged. Cargo fail-fast did not execute the spell-test binary. |
| 0bdf7e62f | AP/follower/dummy follow-up failed compilation before any tests: new oracle used `aura` instead of `AuraEffectState.aura_type`. Log `mastery-init-targeted-followup.log`. | Fixed in faea69f47. |
| faea69f47 | `agent-run mastery-init cargo test --locked -p server -p server-spell-tests --lib --no-fail-fast -j2 -- --nocapture class_completeness_native_mastery_ap_percent_consumes_live_raw_mastery_points native_mastery_follower_learns_universal_base_without_racial_skill_lines dummy8_breath_burn_reduces_only_damage_to_the_owning_monk_and_expires`: server2PASS/13.88s, spell-tests1PASS/8.86s. Log `mastery-init-targeted-green.log`. | None; subsequent changes documentation only. |
| faea69f47 | `agent-run mastery-init cargo check --locked -p server -p server-simulation --tests -j2`: **PASS**,2m47s, zero warnings. Log `mastery-init-check.log`. Run once. | None; subsequent changes documentation only. |
| faea69f47 | `agent-run mastery-init cargo fmt --all -- --check`: **PASS**. Log `mastery-init-fmt.log`. | None; subsequent changes documentation only. |

All logs above live under `/home/osso/.worktrees/logs/`. All builds/tests used `/home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run` after capacity admission. The final follow-up survived an interrupted Pyrun reader: capture was reattached to the **same** cargo PID2809989 before native tests began; no duplicate run. Its complete test output was retained; initial compile-progress output was unavailable.

The two server failures at d96bfa968 were fixture/oracle issues, not further production changes:

- AP: Guardian159195 and Vengeance321299 are secondary passives; the actual spec mastery IDs are155783 and203747. The fixture now reads actual mastery IDs from native spec metadata.
- New follower proof: assuming a full native starter actor had exactly8 total points was wrong (observed12). It now asserts zero mastery rating and exact equality of projected raw mastery to the live aura318 sum, plus114585 knowledge/application and exclusion of race-restricted language668. The bare-player lifecycle test still asserts exactly8 and no duplicate base.

## Per-test disposition: all19 PASS

`server` unless marked otherwise. “Fixture-corrected” means setup only; existing behavioral assertions unchanged.

| Test (unique suffix) | Disposition |
|---|---|
| augmentation_support_native_prescience_cast_grants_only_recipient_crit | Fixture-corrected PASS: learn base114585. |
| augmentation_support_native_sands_changes_actual_recipient_damage_and_healing | Fixture-corrected PASS: learn base114585. |
| augmentation_support_native_sands_snapshots_caster_and_updates_on_its_own_clock | Fixture-corrected PASS: learn base114585. |
| augmentation_support_native_sands_uses_each_aura_caster_and_expires_without_refresh | Fixture-corrected PASS: learn base114585. |
| census_tail_178_native_savant_preserves_missing_mana_and_removes_reversibly | Fixture-corrected PASS: native Arcane spec/known mastery/base; remove Savant before its apply/remove boundary, retain base. |
| holy_echo_of_light_native90_direct_overheal_produces_two_native_ticks_and_expires | Unchanged PASS. |
| holy_echo_of_light_native90_reheal_preserves_unpaid_pool_and_tick_phase | Unchanged PASS. |
| holy_echo_of_light_native90_renew_ticks_do_not_produce_echo | Unchanged PASS. |
| holy_echo_of_light_native90_self_flash_is_eligible_unlike_binding_heals | Unchanged PASS. |
| holy_echo_of_light_native90_two_native_priests_keep_separate_patient_pools | Unchanged PASS. |
| holy_prismatic_echoes_native60_unlearned_bonus_keeps_base_echo_amount | Unchanged PASS. |
| proc_fidelity_262_native_lava_burst_natural_overload_preserves_wrapper_flight | Fixture-corrected PASS: install production StatsPlugin. |
| proc_fidelity_262_native_unselected_eb_overload_cast_mechanism | Fixture-corrected PASS: install production StatsPlugin. |
| proc_fidelity_262_native_unselected_icefury_overload_cast_mechanism | Fixture-corrected PASS: install production StatsPlugin. |
| encounter_coverage_irideus_projected_native_fist_and_surge_damage | Unchanged PASS; available after1fb0230ad merge. |
| class_completeness_native_mastery_ap_percent_consumes_live_raw_mastery_points | Fixture-corrected PASS: real active specs/native mastery IDs/base114585; native AP assertions unchanged. |
| dps_parity_windwalker_combo_strikes_first_cast_and_repeat_without_hit_combo | Unchanged PASS. |
| class_completeness_native_mastery_summon_bonus_tracks_real_owner_stat_changes | Fixture-corrected PASS: learn base114585 alongside BM76657. |
| dummy8_breath_burn_reduces_only_damage_to_the_owning_monk_and_expires (`server-spell-tests`) | Unchanged PASS. |

Additional passing proof: native created-player progression/reapply; follower universal mastery and restricted-language exclusion; three-class zero-rating base/unlearn/relearn; base+bonus requires the known active-spec mastery (foreign mastery does not enable it); timed mastery refresh; ordinary-NPC exclusion;11 race/class controls.

The four supplied master-baseline follower failures were not rerun or modified. No full-suite or numerical DPS-parity claim; lead integration gate remains separate.
