# Pet battle turn engine

Pure deterministic server library in `crates/server/src/pet_battle/`, exported through `server::pet_battle`. [Mechanics, schema and source provenance](../wiki/systems/pet-battle-engine.md); [full property coverage](../pet-battle-effect-coverage.md).

## What it must do

- [x] Load the ten requested battle DB2 tables through its own strict SQLite loader, with test-only local-CASC CSV fixtures and no journal-code dependency/runtime CSV fallback.
- [x] Derive stats from species/breed states, DB2 quality and level; preserve float32 rounding boundaries and fail invalid inputs explicitly.
- [x] Model three pets per team and an active slot; ability/swap/pass/forfeit decisions; speed order and injected tie RNG; faint replacement without combat-round advancement.
- [x] Dispatch common damage/heal/state/aura properties, including accuracy, variance, critical/type modifiers, timed ticks/expiry, refresh, multi-turn continuation and cooldowns.
- [x] Trigger Beast, Flying, Critter, Magic, Mechanical, Undead, Humanoid, Aquatic and Dragonkin passives through BattlePetState names; use actual ability descriptions/effect values for numeric rules.
- [x] Reject unsupported properties, parameters, events and special aura states explicitly; test rejected actions preserve RNG.
- [x] Assert concrete real-ID behavior: Bite110, Burn113, Healing Wave123, Poison Fang152, Burrow159, Deep Breath169, Curse of Doom218, Grasp249, Wish273, Time Stop600 and Recovery726.
- [x] Publish104-ID row/implementation/test coverage, distinguishing static admission from behavioral proof.
- [ ] Prove exact damage rounding/type weak multiplier and control/event timing against captured Retail runtime or current client game tables.
- [x] Extend data-grounded dispatch with failure/state branching, drain, ramping attacks, priority, conditional/percentage damage, split-team attacks and global weather.
- [x] Execute Cauterize's last-hit heal, Comeback's lower-health bonus, below-quarter-health attacks, charged two-use attacks, state-guaranteed accuracy and family-filtered healing of other allies from authenticated DB2 descriptions.
- [x] Route authenticated three-row enemy-team attacks to the active and other slots; execute first-strike interrupts, state-boosted stun chances, bonus-state attacks and Leech Life's bounded post-hit heal.
- [x] Keep Cyclone189/190 bound to the enemy team through swaps and removable by object reset; apply Stench527/1010 to each enemy pet. These do not establish other object or159 selector semantics.
- [x] Execute strict speed bonuses for Slow Digestion/Rapid Consumption, Creeping Fungus's Moonlight bonus, Weakening Blow/Superbark's one-health floor and Eat the Fly's low-health heal bonus.
- [x] Execute the seven sourced two-way walls325/433/444/465/479/2318/2398 with two charges shared between allied and enemy direct attacks, owner-team swap persistence, refresh and object cleanup. Keep Onyx439's distinct friendly-prevention semantics rejected rather than treating it as identical.
- [x] Lock-On301's first use marks the enemy with one-round Locked On300; another use against the marked active target launches ordinary direct Mechanical damage and consumes the mark. An unused round expires the mark, a different active target needs a new lock, and misses preserve ordinary accuracy rules. Preserve75/76 self-charge behavior; keep GM722/other77 tuples rejected. Cross-caster sharing and full Retail timing/VFX remain uncertified.
- [x] Perk Up576/Healthy577, Ancient Blessing611/610 and Ancient Knowledge1991/1990 increase maximum health by authored5/5/10 HP per actual pet level, before their ordinary healing; refresh without stacking and remove capacity/cap current HP on expiry. Require explicit validated level1..25 from the encounter caller, retain existing percentage/floor convention and owned-health persistence cap, and reject other state2 values/owners including Stockpile/SwapLock. Independent Retail percentage ordering remains unqualified.
- [x] Life Exchange277 equalizes the two active pets' absolute HP as a direct health operation, capped at maximum health; misses preserve both values and reserves remain untouched. Keep other67 tuples rejected. Integer averaging follows existing HP conventions, not independently certified Retail rounding or proc/feedback behavior.
- [x] Launch Rocket293 creates its caster's ready rocket294 on first use without attacking the enemy; second use launches ordinary typed damage and consumes the ready state, including on a miss. Another launch requires another setup. Keep other75 parameter/target routes rejected; object-sharing and exact Retail timing remain uncertified.
- [x] Armageddon1025 damages every living enemy pet through the ordinary typed attack path, then kills its user without activating Mechanical Failsafe. Missed enemy attacks must not cancel the sacrifice, dead enemy reserves must remain dead, and allied reserves must be conserved. Keep other unqualified kill/clone/reverse graphs rejected.
- [x] Haymaker762 applies its one-round self-stun and Blingtron Gift Package989 heals its user only when their own attack misses. A blocked or successful attack and another pet's miss must not trigger these outcomes. Preserve ordinary self-control immunity, healing caps and authored loadout validation; keep other194/159 graphs rejected.
- [x] Refuge1344 shields its user and each other living friendly Critter with independent one-hit Block759 charges for six rounds; preserve original caster, refresh, swap ownership and direct-hit-only block behavior through the shared aura path. Keep other223 selectors rejected.
- [x] Dragon's Call2283 counts additional same-species pets in the caster's three-pet roster, excludes caster/enemies and uses the existing direct-damage pipeline. Carry explicit validated species identity into every Pet; never infer species from family. Keep test-only370 roots2259/2280 rejected.
- [x] Raise Dead2347 and Mass Resurrection2348 revive only dead allied reserves at5% maximum health, clear dead/expired-undead metadata without resetting once-per-battle family resurrection, and recognize persisted zero-HP owned pets as dead on encounter load.
- [x] Execute Fade1084's sourced self-owned end-of-round random swap after its dodge expires; select only living reserves and keep other107 target-selection semantics explicitly gated.
- [x] Execute Nevermore522 and Spell Gulp1858's wearer-owned next-ability lockdown and self-removal through bounded event9; keep slot lockdown separate from cooldown and tick bench locks. Mind Games2390 locks the choosing pet's one-based slots1/2 without inventing stun.
- [ ] Generalize reactive damage/heal/ability-use callbacks only when their causal ordering, target/source ownership and recursion are established; bounded Leech Life383 and Nevermore/Spell Gulp do not certify other graphs or persistent Disruptive Spores2213.
- [x] Weather must affect bench pets, replace prior weather, remove exactly applied state deltas, and preserve Elemental negative-weather exemptions.
- [ ] Reach at least90% executable effect-row coverage; retain explicit rejection and document undetermined parameters/states.

## How it works

- [Engine mechanics, schema contract and source boundaries](../wiki/systems/pet-battle-engine.md).
- [Property matrix, proof levels and reproduction](../pet-battle-effect-coverage.md).

## Implementation inventory

- `src/lib.rs`, `pet_battle/mod.rs` — standalone library exports, no server startup wiring.
- `pet_battle/catalog.rs` — own typed SQLite loader, raw unresolved-state retention and admission inventory.
- `pet_battle/stats.rs`, `model.rs` — stats and battle/team/pet/aura/action/event representation.
- `pet_battle/rules.rs`, `engine.rs`, `effects.rs` — opcode admission, deterministic round sequencing and effect execution.
- `pet_battle/tests.rs`, `property_tests.rs` — isolated shipped-table fixtures, real ability IDs, fixed RNG and hand-derived behavioral assertions.

## Tests asserting this spec

`crates/server/src/pet_battle/{tests,property_tests}.rs`; native `agent-run pbfx cargo test --locked -p server pet_battle`. Tests copy battle tables from shipped world.db, without CSV environment overrides. Exact revisions, counts, format/check evidence and remaining parity boundaries are in the [coverage report](../pet-battle-effect-coverage.md).

## Known gaps (current cycle)

- [x] Runtime DB producer imported the ten battle tables; tests read the shipped SQLite tables without mutating them.
- [ ] Independent verification could not run because Claude OAuth expired; no paid substitute used.
- [x] Assert Elemental negative-weather immunity and replacement/expiry on all six slots with Darkness256 and Moonfire595.

## Out of scope

This effect-expansion cycle does not change networking, wild spawning, HUD, capture/XP, journal persistence, deployment or protected services. The [wild runtime](wild-pet-battles.md) already consumes this library; effect/parity coverage remains bounded.
