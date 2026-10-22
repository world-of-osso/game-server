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
|Thunder Clap Rend|Missing|RED pending|
|Shield Charge chain|Missing Dummy producer; generic effect96 exists|RED pending|
|Whirling Blade/Ravager|Missing named producer/ticks|RED pending|
|Phalanx/Bloody Rebuke|Missing named producer/consumer|RED pending|
|Guillotine/Venomfang|Equipped driver lifecycle exists; damage producers missing|RED pending|
