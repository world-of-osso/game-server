# Combat pet lifecycle

Retail Mainline behavior applies to every client art skin. Hunter taming is existing behavior, not proof of the remaining lifecycle.

## Requirements

- An actual attack/hit against a controlled unit also engages its direct owner before lethal damage cleanup. Threat-capable owners receive zero-amount native engagement, never transferred pet damage threat. Missile launch alone retains target-side suppression and does not engage that owner. The shared rule covers melee, direct damage and missile impact; it does not change follower tank/healer policy or pet stats.
- Hunter Call Pet 1–5, Dismiss, Revive, Mend and locally supported Feed Pet operate on the character's owned pets. A dead pet stays dead until revived; relogging must not heal it.
- Stable Master interaction exposes the Retail active/stabled slots, moves/swaps, rename, abandon and pet specialization selection. Slot capacity and spell identities come from the installed Retail data/UI, never estimates.
- Character-scoped redb storage preserves health, name, specialization and summoned/dismissed state through logout/relog and death. Existing stable records migrate without losing identity, slots or stance.
- Warlock summons use the authored demon template for each supported local summon spell. Dismiss keeps its record; relog restores the current demon. Fel Domination is conditional on authentic data support.
- Pet frame/action bar reflect the current owned pet. Actual pet constructors and source Control=PET publish `UnitIsPet(true)` alongside owner identity; player-controlled ally guardians do not. Local and party frames must not infer pet designation from ownership/flags. [Shared identity contract](../../../shared-protocol/docs/specs/pet-identity.md). Forever changes art only.

## Evidence boundary

Acceptance needs behavioral persistence/cast/ownership tests and inspected native live captures in both skins. A successful cast alone is not proof of healing, revive or stable state mutation.
