# Typed invisibility and quest-area auras

Authority: TrinityCore a352b1fa Object.cpp `CanDetectInvisibilityOf`, SpellAuraEffects.cpp `HandleModInvisibility` / `HandleModInvisibilityDetect`, SpellMgr.cpp `SpellArea::IsFitToRequirements`; TDB1210.26091 `spell_area`.

- Aura18 carries its authored invisibility type (MiscValue0) and summed integer amount, including stacks. Aura19 carries independent detection types/values; zero value still marks a type present.
- An observer detects a unit only when every hidden type has matching detection with at least its hidden value. Invisible observers also require reciprocal type detection, except aura335.
- Visibility/replication and hostile/friendly spell targeting and attacks use observer-specific detection. Detection changes re-evaluate stationary observers and targets; late arrivals follow the same policy. Self remains visible.
- Authored spell_area requirements use zone/area, quest status masks, signed aura prerequisite, race mask and gender. Autocast applies eligible auras; ineligible managed auras are removed. Quest changes and area movement update eligibility.
- No authored invisibility aura is stripped to make quests playable. No canonical world database changes are part of this feature branch.

Acceptance: real Jaga/Vin and Mountain-Lord Rendan visibility eligibility, native quest credit/loot/turn-in with Modern and Forever skins on a private realm; targeted RED/GREEN and check/fmt evidence. Generic scripted phasing is not included.
