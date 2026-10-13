# Scripted jump movement

Source-authenticated `MotionMaster::MoveJump` subset used by Gordoth, from TC a352b1fa. [System](../wiki/systems/boss-scripts.md).

## What it must do

- [x] Move between source positions on a two-point linear spline plus parabolic elevation, using the explicit speed capped at50 and source gravity/min-height/default max1000. Duration starts at1ms plus the truncated segment travel time.
- [x] Emit effect-motion arrival only at the endpoint; Gordoth sets landing facing, destroys its three nearby vials and its seven researchers cast the sourced gore sequence/Bloody Suicide120022, whose immediate periodic51744 instakill causes real self-deaths without boss credit.
- [ ] Highest-priority ownership blocks lower motion throughout the landing tick and releases on the next unit update. Code is wired; independent competing-driver proof remains open.
- [ ] Death interrupts a running jump without an arrival callback. Code is wired; interruption proof remains open.

## How it works

- [Boss scripts](../wiki/systems/boss-scripts.md).

## Implementation inventory

- `scripted_ai/jump_movement.rs`: source geometry and motion lifetime.
- `scripted_ai/mod.rs`: effect-motion hook/scheduling.
- `creature_ai.rs`, `scripted_ai/movement.rs`, `scripted_ai/waypoint_path.rs`: suspend lower movement.
- `scripted_ai/ragefire_chasm.rs`: source jump/landing callbacks.
- `spell_cast/effects.rs`: generic instakill death path needed by Bloody Suicide.

## Tests asserting this spec

- `scripted_ai/tests/data_bosses.rs`: `encounter_coverage_gordoth_source_jump_lands_breaks_vials_and_researchers_die`, real map389 population/objects/spells.

## Known gaps (current cycle)

- [ ] Original spawn-independent home/facing, static AT7899/GM handling, animkit, cage task and native jump/GO animations.
- [ ] Source Gordoth UpdateAI never pumps its scheduled +30ms cage task; no synthetic pump is introduced.
- [ ] Instakill-specific packet/player CHEAT_GOD state are unmodelled; current proof is the researcher NPC self-kill chain.

## Out of scope

Transport/facing-target/spell-curve jump variants and a generalized replacement of every movement generator; not required by the sourced Gordoth call.
