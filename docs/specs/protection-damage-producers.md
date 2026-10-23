# Protection damage producers

## Scope

Restore the missing spec73 producer channels identified by `game-engine/data/diagnostics/dps-parity-2026-10-21/top10-unobserved-channel-routing.{md,json}`. This is one Protection batch; Feral103 follows separately. Demolish and Lunar Inspiration belong to13540; Fire belongs to13122. Do not duplicate already-published kit entries or turn triggered children into standalone rotation actions.

## Required behavior

- Thunder Clap/Thunder Blast apply the native Rend aura to each landed enemy when the caster knows Rend, following pinned TC `spell_warr_thunder_clap_rend`; preserve slow, damage, owner and recipient exclusions.
- Shield Charge's Dummy casts native385953, whose existing effect96 movement/landing dispatch casts385954. Preserve original movement, hit timing, primary/area damage, stun and resources.
- Whirling Blade produces its native Ravager channel through existing proc/RPPM machinery; realize the timed Ravager damage children without rewriting source periods, coefficients or proc rates. Use a named SimC policy only for behavior absent from TC and DB2.
- Phalanx's native buff producer and Shield Slam consumer emit1270116 once per eligible consumption. Bloody Rebuke follows its actual learned/equipped set eligibility and native Ravager/Revenge inputs.
- Equipped Guillotine/Venomfang producers use native item-scaled driver amounts, proc eligibility and the authenticated set relationship; keep base versus perfected variants and missing-health scaling distinct. Do not grant procs from catalog presence or invent source amounts.

## Sources and proof

- TC a352b1fa: Thunder Clap Rend `spell_warrior.cpp:2122–2144`; Shield Charge `1773–1789`.
- Canonical child graph `game-engine/data/diagnostics/tc-script-child-edges-a352b1fa.json`, SHA256 `b450b7281be43372705bf78e52bece6c3420df0c78573c921cb8711478f231e3`, plus current native TriggerSpell closure. Preserve `unresolved_calls`; no graph edge proves execution.
- Named policy reference: SimC76f8118 `sc_warrior.cpp` and `unique_gear_midnight.cpp`; DB2 wins for current values. Source-specific gaps remain explicit rather than synthetic child actions.
- Each producer requires native behavioral RED→GREEN, with actual damage/aura/resource receipts and ownership, absence/removal and timing controls. Targeted success does not establish full legal-loadout, UDP or DPS parity.

## Current coverage

|Producer|Status|Proof|
|---|---|---|
|Thunder Clap Rend|TC hit hook implemented; native damage/ownership qualified|5e69c75db both Clap cases PASS; charge case remains separate|
|Shield Charge chain|Native movement/landing/area/stun qualified|13906f459 PASS with sourced display/reach/template metadata; valid RED59677be35 retained|
|Whirling Blade/Ravager|Missing named producer/ticks|5e69c75db native Ravager fails missing156287 tick; Whirling Blade fails missing228920 after40 real primary-target casts|
|Phalanx/Bloody Rebuke|Named Phalanx eligibility/consumer implemented; GREEN pending; Bloody Rebuke missing|5e69c75db produces1278009 but fails consumption; absence and foreign-buff controls pass; 7327a4c03 non-Clap actual-hit RED0PASS/1FAIL47.10s: Shield Slam wrongly produces the buff; Bloody Rebuke RED pending|
|Guillotine/Venomfang|Equipped driver lifecycle exists; damage producers missing|RED pending|

### First TC producer receipts

Ignored a0a9fca65 `protection_producers_tc_ --include-ignored`:1PASS/2FAIL16.79s, compile7m59s/no warnings. Shield Charge admits with actual2362 shield but remains at its initial position; landed Thunder Clap retains its slow but lacks388539. Native unknown-Rend control passes. `/home/osso/.worktrees/logs/census-tail/protection-tc-red.log`. Minimal TC hooks retain native charge flight/trigger child and qualify Rend by actual KnownSpells, not presence of384277 or a generic unconditional bleed. First GREEN attempt5e69c75db:2PASS/1FAIL12.22s, compile8m08/no warnings (`protection-tc-green.log`). Both Clap cases pass, including actual Rend tick damage; Charge still cannot move because its fixture NPC has no ModelDisplay/ModelReaches. Correct the fixture from actual template6/display10913 and catalog reach, remove the unqualified Charge hook, and obtain valid RED before restoring it. No runtime charge fallback or authored reach change. Corrected RED59677be35:0PASS/1FAIL6.94s, compile4m52/no warnings (`protection-charge-red.log`), actual reach prerequisite passes and missing TC hook fails movement. Restore only the pinned Dummy→385953 bridge. Subsequent compiled7327a4c03 moves far enough to reach native landing damage, then panics at hit.rs407 because the fixture lacks WorldData for the native NPC defense reader (8.55s). Supply actual template6 through the native fixture helper; do not change the runtime defense reader. GREEN pending.

Compiled ignored policy cases5e69c75db:2PASS/3FAIL16.92s (`protection-policies-red.log`); Phalanx's native proc already produces1278009 but Slam does not consume it; absence/foreign-buff controls pass. Ravager lacks156287 periodic damage; Whirling Blade lacks228920 after40 real Devastate impacts with actual5RPPM/bad-luck timing. No flags/chances/RNG/proc timers modified. Non-Clap actual-hit RED7327a4c03:0PASS/1FAIL47.10s, compile6m31/no warnings (`phalanx-eligibility-red.log`): Shield Slam itself incorrectly produces1278009.

Named `PHALANX_CLAP_ONLY` policy restricts the native proc to ordinary6343/435222 impacts without changing its native mask, chance or child. Named `PHALANX_SHIELD_SLAM_CONSUMPTION` runs once at actual Shield Slam execution, casts native1270116 toward that cast's explicit target, then consumes the caster's1278009 buff. Pinned TC has no Phalanx/Shield Slam script and DB2 has no1270116 trigger edge; SimC76f8118 supplies only these missing decisions. 13906f459 targeted hooks run:6PASS/1FAIL/2IGNORED15.33s, compile7m00/no warnings (`protection-hooks-green.log`). All three TC cases and three Phalanx absence/foreign/non-Clap controls pass. Phalanx consumption and real child damage assertions pass, but its second cast incorrectly waits7s for the native9s cooldown and fails OnCooldown. Correct only fixture advancement to10s; rerun only that affected case, retaining six disjoint passes. Remaining producer rows stay open.
