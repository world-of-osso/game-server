# Scripted jump movement

Source-authenticated `MotionMaster::MoveJump` subset used by Gordoth, from TC a352b1fa. [System](../wiki/systems/boss-scripts.md).

## What it must do

- [x] Move between source positions on a two-point linear spline plus parabolic elevation, using the explicit speed capped at50 and source gravity/min-height/default max1000. Duration starts at1ms plus the truncated segment travel time.
- [x] Emit effect-motion arrival only at the endpoint; Gordoth sets landing facing, destroys its three nearby vials and its seven researchers cast the sourced gore sequence/Bloody Suicide120022, whose immediate periodic51744 instakill causes real self-deaths without boss credit.
- [x] Gordoth records landing home/facing independently of immutable spawn coordinates; home return honors that landing position/orientation. Respawn resets the explicit home before InitializeAI.
- [x] Suspend an existing point generator while the jump owns movement, then resume it. Real Mortimer POINT1's delayed self-death observes both boundaries, without querying controller class selection.
- [ ] Landing-tick handoff against every lower driver remains unproved; code retains ownership until the next unit update.
- [x] Gordoth's mid-jump death suppresses arrival effects: vials/researchers remain unchanged while the real boss death completes.

## How it works

- [Boss scripts](../wiki/systems/boss-scripts.md).

## Implementation inventory

- `scripted_ai/jump_movement.rs`: source geometry and motion lifetime.
- `scripted_ai/mod.rs`: effect-motion hook/scheduling.
- `creature_ai.rs`, `scripted_ai/movement.rs`, `scripted_ai/waypoint_path.rs`: suspend lower movement.
- `scripted_ai/ragefire_chasm.rs`: source jump/landing callbacks.
- `spell_cast/effects.rs`: generic instakill death path needed by Bloody Suicide.

## Tests asserting this spec

- `scripted_ai/tests/data_bosses.rs`: real map389 parabola/landing/self-death, jump interruption and real Mortimer point-arrival lifecycle.

## Known gaps (current cycle)

- [x] Ordinary-player AT7899 uses source rotated-box entry, same-copy boss lookup and per-instance OnlyOnce; a missing boss leaves a later entry retryable.
- [ ] GM-mode handling, animkit, cage task and native jump/GO animations. No account-role-to-GM inference.
- [ ] Source Gordoth UpdateAI never pumps its scheduled +30ms cage task; no synthetic pump is introduced.
- [ ] Instakill-specific packet/player CHEAT_GOD state are unmodelled; current proof is the researcher NPC self-kill chain.

## Out of scope

Transport/facing-target/spell-curve jump variants and a generalized replacement of every movement generator; not required by the sourced Gordoth call.
