# Charm and possession

Native aura2/6 retain the existing unit rather than summoning/replacing it. `CharmState` saves the original faction and affected flag/control state; `UnitCharm` replicates the charmer's server identity and direct-possession mode. The charmer's server `CharmedUnit` link authorizes commands and input. [Contract](../../specs/charm-possess.md).

## Lifecycle

Application stops casts and combat, clears the old threat projection, dismounts, adopts the charmer's faction and installs the existing pet command bar. Aura6 follows and assists its charmer; player-cast aura2 suppresses autonomous chase and locks the player's body. NPC-cast aura2 is charm, matching TC. The original scripted/roaming AI does not tick while charmed.

Possessed input reuses PlayerInput with the controlled unit's epoch and existing server movement bank/terrain policy. Ordinary charm does not redirect the charmer's mover. An independently controlled player's own connection cannot drive that player while charmed. Aura cancellation/removal restores identity and control; pet abandon releases rather than despawns a charmed unit. Existing aura duration/channel/interrupt machinery remains authoritative.

## Attack authority

Pet/charm attacks use the canonical spell-target enemy predicate, including duel/arena rules, rather than a second faction-only check. Permission controllers resolve UnitCharm before summon ownership; movement/range positions remain the actual unit's. Both melee and spell nonlethal-duel checks use the controlling principal (TC Unit.cpp930–932). A charmed player's original connection cannot stop the NPC-charmer's active melee. Actual dealer identity remains the controlled unit in combat events.

## Native70448 source qualification

605 Mind Control effect0=aura2;205364 Dominate Mind,1098 Subjugate Demon and30923 Domination effect0=aura6.605 and30923 have aura interrupt flags0/0;205364 and1098 have0/65536 (TC SpellAuraInterruptFlags2::SeamlessTransfer,NYI), not damage-break flags.51514 Hex has56/60/260/321 and is excluded. No synthetic damage-break bit or class-name whitelist.

Canonical `tdb_creature_template_spell` supplies NPC spell slots (TDB1210; CreatureID/Index/Spell/VerifiedBuild). Imp416 carries3110 in source slots0/2; duplicate spell IDs produce one button. Ordinary charm reads source slots0–3; possession reads0–7. Passive spells are cast; unavailable-while-charmed spells omitted; possession spells are manual and ordinary explicit/autocastable spells use authored default autocast flags. Kobold6 has no rows: per-creature `CHARM_CREATURE_SPELL_BAR_SOURCE_ABSENT`, commands still available. Missing native SpellInfo gets `CHARM_SOURCE_SPELL_ABSENT_NATIVE70448`, never a substitute. The earlier global absent-table gap was incorrect (checked the unprefixed name only).

An idle creature publishes MovementSpeed0 for animation, not a movement allowance. Taking possession publishes its actual control run speed before the first client report; otherwise the native client cannot start moving. The mover epoch advances beyond both the target's and charmer's current epochs, rejecting old body reports.

NPC-controlled players retain their Player component but take the NPC faction-attack side while charmed; PLAYER_CONTROLLED is set only for a player charmer. Combat, pet-AI and spell target predicates consume that distinction. Maker30923 now follows TC's20–30s/repeat20–30s with a multiple-player threat gate and random-player target; ordinary single-player pulls do not cast it. Contract records proof status, not an implied completeness claim.

## Sources

- TC a352b1fa `Unit.cpp11837-12130,6460-6550`; `SpellAuraEffects.cpp3238-3320`; `CharmInfo.cpp49-165`; `PassiveAI.cpp28-76`; `PlayerAI.cpp634-653`.
- Local native world.db SpellEffect/SpellInterrupts rows605/1098/205364/30923/51514, read2026-10-21.
- [Pet control](../../specs/pet-control.md) — reused command/bar/AI semantics and existing project deviations.

## See Also

- [Aura runtime](auras.md)
- [Factions](factions.md)
