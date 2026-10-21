# Charm and possession

Native aura2/6 retain the existing unit rather than summoning/replacing it. `CharmState` saves the original faction and affected flag/control state; `UnitCharm` replicates the charmer's server identity and direct-possession mode. The charmer's server `CharmedUnit` link authorizes commands and input. [Contract](../../specs/charm-possess.md).

## Lifecycle

Application stops casts and combat, clears the old threat projection, dismounts, adopts the charmer's faction and installs the existing pet command bar. Aura6 follows and assists its charmer; player-cast aura2 suppresses autonomous chase and locks the player's body. NPC-cast aura2 is charm, matching TC. The original scripted/roaming AI does not tick while charmed.

Possessed input reuses PlayerInput with the controlled unit's epoch and existing server movement bank/terrain policy. Ordinary charm does not redirect the charmer's mover. An independently controlled player's own connection cannot drive that player while charmed. Aura cancellation/removal restores identity and control; pet abandon releases rather than despawns a charmed unit. Existing aura duration/channel/interrupt machinery remains authoritative.

## Native70448 source qualification

605 Mind Control effect0=aura2;205364 Dominate Mind,1098 Subjugate Demon and30923 Domination effect0=aura6.605 and30923 have aura interrupt flags0/0;205364 and1098 have0/65536 (TC SpellAuraInterruptFlags2::SeamlessTransfer,NYI), not damage-break flags.51514 Hex has56/60/260/321 and is excluded. No synthetic damage-break bit or class-name whitelist.

Canonical world.db lacks creature_template_spell. `CHARM_CREATURE_SPELL_BAR_SOURCE_ABSENT` records unavailable authored NPC ability buttons; Attack and ordinary charm commands are usable without invented spell loadouts. Tests/consumer and native proof status live in the contract, not an implied completeness claim.

## Sources

- TC a352b1fa `Unit.cpp11837-12130,6460-6550`; `SpellAuraEffects.cpp3238-3320`; `CharmInfo.cpp49-165`; `PassiveAI.cpp28-76`; `PlayerAI.cpp634-653`.
- Local native world.db SpellEffect/SpellInterrupts rows605/1098/205364/30923/51514, read2026-10-21.
- [Pet control](../../specs/pet-control.md) — reused command/bar/AI semantics and existing project deviations.

## See Also

- [Aura runtime](auras.md)
- [Factions](factions.md)
