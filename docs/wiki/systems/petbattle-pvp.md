# PvP pet battle project policies

Verified: 2026-10-21. [Contract](../../specs/petbattle-pvp.md). Implementation pending; behavioral RED tests prepared.

## Explicit project policy, not Retail numerical authority

Lead decision on 2026-10-21 supplies the previously unsourced rules:

- **PET_BATTLE_PVP_TIMERS:** proposal, initial selection, ordinary decision and faint-replacement windows are each30seconds. Proposal nonresponders decline/leave; accepted opponents retain their queue wait. Initial timeout keeps the current lead; ordinary timeout passes; replacement picks the first living slot. Three consecutive ordinary turn timeouts forfeit; a submitted ordinary decision resets that owner's streak.
- **PET_BATTLE_PVP_MATCHMAKING:** FIFO pairing, average team-level distance at most3, widening by1 per10seconds waiting. No rating. Compare integer team-level sums to avoid rounded averages. Older waiting participant's allowance controls pairing; earliest compatible opponent wins.
- **PET_BATTLE_PVP_XP:** reuse existing wild XP calculation against opponent levels, living participating winners only. Losers/dead/nonparticipating/max-level pets receive no XP. Multi-opponent aggregation is being confirmed with lead; no separate Retail reward claim.

The `BattlePet*`70448 tables provide authentic species/abilities/states, not these timer or matchmaking constants. No table mutation or invented pet data is authorized.

## Retail sources

Local cache `/home/osso/.cache/wow-ui-sim/blizzard-ui/retail/AddOns/`:
- `Blizzard_PetBattleUI/Shared/Blizzard_PetBattleUI.lua:185–199,479–528`: mandatory front selection, C API turn timing, waiting caption,337px proportional timer fill. No numerical duration supplied.
- `Blizzard_PetBattleUI/Shared/Blizzard_PetBattleUI.xml:1374–1427` and `Blizzard_PetBattleUIPatchwerks.xml`: timer size/anchors/art and Pass template.
- `Blizzard_StaticPopup_Game/Mainline/StaticPopupSpecial.xml:23–54`: Accept/Decline immediately hide the ready popup; acceptance availability comes from C API.
- `Blizzard_Collections/Shared/Blizzard_PetCollection.lua:1677–1687`: queued/proposal/suspended Leave Queue states.

Source hashes and exact70448 BattlePet export hashes: engine `data/diagnostics/pb-pvp-2026-10-21/sources.json`. Existing [battle engine](../../pet-battle-effect-coverage.md) remains the combat authority. Journal-card pad normalization remains unchanged in the engine.
