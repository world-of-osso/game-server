# Meeting stones

Summoning a party member at a meeting stone, following TrinityCore (`GameObject::Use`, `Spell::EffectTransmitted`, `Player::SendSummonRequestFrom` / `SummonIfPossible`). Messages use `InteractionChannel` (shared-protocol `SummonRequest` / `SummonResponse`). The client popup is specified in game-engine `docs/specs/meeting-stones.md`; for how it works see [meeting stones](../wiki/systems/meeting-stones.md).

## What it must do

### Stones
- [x] Meeting stones (`content_gameobject` rows of type 23 on maps 0 and 1) spawn with the other game objects.
- [x] `UseGameObject` on a stone (alive, within 5 yd of the model box) opens no frame and casts the stone's spell as the user: Summoning Stone Effect (59782), or Ritual of Summoning (61994) at the Summoning Portal (194097).
- [x] The user's target must be another player in its party or raid, else `CastFailed` `InvalidTarget`.
- [x] A stone with ContentTuning needs the user and the target at its max level (`LevelRequirement` / `TargetTooLowLevel`); a stone without ContentTuning (178834, near the Deadmines) has no level check.
- [x] A target with an open summon offer is refused (`SummonPending`).

### Ritual
- [x] 59782 casts for 5 s; walking does not break the cast (InterruptFlags 0). On completion the summoner channels for 2 min and the Meeting Stone Summoning Portal (179944) appears at the summoner, facing its way.
- [x] Movement input or stopping the channel removes the portal; so does the channel's end.
- [x] Another member of the summoner's party clicking the portal completes the two-caster ritual: the channel ends, the portal goes, and the summoner casts 7720 (SUMMON_PLAYER) at its target. The summoner's own click and clicks from outside the party do nothing.

### Offer
- [x] The target gets `SummonRequest { summoner, zone_id, time_left_ms: 120000 }` and holds the summoner's position at that moment for 2 min. `zone_id` is the top-level zone at the summoner's position: reuse city rest's `GroundMap::area_at` and `AreaTable` ParentAreaID traversal, not a player `Zone` component. Retail `GetConfirmSummonExpiryText` uses `C_SummonInfo.GetSummonConfirmAreaName` (cached `Blizzard_StaticPopup_Game/GameDialogDefs.lua:2402-2409`); TrinityCore `TerrainInfo::GetZoneId` follows parents to the zone.
- [x] Accepting in time ends any taxi flight and moves the target to the held position. Declining, or accepting after 2 min, ends the offer without moving.
- [x] Responses while dead or in combat are ignored; the offer stays open.

## Gaps
- The summoner must target the player, and only players in its interest radius can be targeted (client); an accepted summon crosses maps through a map transfer ([instances](instances.md)).
- The ritual's `animSpell` (32783 Arcane Channeling) is not cast on helpers.

## Zone regression
`meetingstones_summon_from_stormwind_subzone_sends_zone_without_player_zone_component` uses real terrain at WoW (-8850, 650): MCNK Trade District 5148 resolves to Stormwind City 1519. A player without `Zone` sends the exact `SummonRequest` over the existing host message path. Earlier tests injected `Zone`, masking the live failure.

## Live evidence (2026-09-26)
Own server :5080 with three game-engine clients at Blackrock Mountain stone 179584 (ContentTuning 201, level 30): stone use opened portal 179944 after the 5 s cast (`Spell 59782: ritual 179944 … opened`), the second member's click sent the offer, and the accept moved the target 88 yd to the summoner (`… accepted a summon`). Details in game-engine `docs/specs/meeting-stones.md`.
