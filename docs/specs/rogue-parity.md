# Native Rogue parity mechanics

Assassination259 combat and the shared Rogue attack-speed path follow pinned SimC76f8118 behavior with native values under the [DPS parity contract](dps-parity.md). Runtime code lives in `crates/server-combat/src/auras/rogue.rs` and `spell_cast/scripts/rogue_{children,hero}.rs`. Shared coating and Venomous Wounds requirements remain in the parent contract; [auras](../wiki/systems/auras.md) and [casting](../wiki/systems/spell-casting.md) describe the native runtime.

## What it must do

### Poison and finisher state

- [x] Positive lethal-poison direct hits against the caster's active385627 Kingsbane grant native394095 stacks. Periodic poison ticks, other casters and unmarked targets do not qualify. Native amounts/cap determine periodic damage; the ramp ends with its owned dot. Source: `sc_rogue.cpp2948–2954,4975,11155–11156`.
- [x] Envenom snapshots Amplifying Poison's native bonus at its native owned383414 stack threshold, then consumes exactly that threshold before generating new poison. Preserve other casters' stacks and sub-threshold state. Source: `sc_rogue.cpp4281–4310`.
- [x] Selected14190 rolls its live effect0 chance on critical CP-builder hits, including Fan of Knives and both Mutilate hands, and casts native effect1 child14189. White swings, noncrits, periodic ticks, Pistol Shot, Shuriken Toss and Poisoned Knife do not qualify. Do not repeat the proc through deferred dispatch. Source: `sc_rogue.cpp2178–2180,4471–4473,5085–5086,5137–5138,5933–5934,6351–6352,7921–7937`.

- [x] Kingsbane follows pinned default `bugs=1`: apply its extra native Potent Assassin direct bonus and extra native Implacable effect1 direct/effect2 periodic bonus, in addition to ordinary native family-mask modifiers. Read live passive amounts; preserve coefficients, costs, cadence and earned ramp. Source: `sc_rogue.cpp4921–4945`.

- [x] Each landed native Mutilate hand funds394021 with its resolved damage times selected381673 effect0's native fraction. Native cadence, duration, rolling carry and no-second-crit flags remain authoritative; dynamic Lethal Dose applies to its ticks. Source: `sc_rogue.cpp5131–5135,6278–6287,7993–7999`.

### Stealth and bleed children

- [x] Selected381632 makes Garrote retain392401 effect1's native bonus when applied during real stealth or the native post-stealth window. Leaving the last real stealth aura grants392401. Readiness and recovery storage both use zero Garrote recovery during those windows, including the first stealth cast; preserve real costs and GCDs. Source: `sc_rogue.cpp4550–4561,4604–4609,7351–7378,11121–11133`.
- [x] Successful Rupture with selected381627 applies native381628 at its captured effective-CP multiplier, minimum1, without another payment. Preserve native coefficients, cadence and duration. Source: `sc_rogue.cpp5257–5264,6200–6203`.
- [x] Successful Kingsbane with selected1265387 starts1265787; each native effect1 tick casts1265794 and1265795 at its retained impacted target. Target loss does not select a replacement; removing the driver clears its target. Source: `sc_rogue.cpp4891–4905,4954–4958`.

### Assassination Deathstalker

- [x] Native Garrote establishes one owned457129 mark using selected457052 effect0 stacks. Transfer it off the previous target without refreshing the same target's existing mark. Finishers meeting effect1's native CP threshold consume one stack and deal457157 damage. Source: `sc_rogue.cpp4599,8732–8817`.
- [x] Depletion with selected457058 grants457280 after1ms, including native Energy. A maximum-CP Envenom retains its native damage/critical bonus, reapplies the native mark count and consumes Darkest Night. Lower-CP casts consume neither marks nor Darkest Night while it is active. This lifecycle qualification is spec259-only. Source: `sc_rogue.cpp2435–2440,2569–2574,8732–8817`.

- [x] Selected457054 dispatches native457193 once per landed white swing against its actual victim while that victim has the caster's own active457129 mark. Retargeting to an unmarked victim must not dispatch against another target's mark. Native payload data supplies damage. Source: `sc_rogue.cpp3411–3419,10282,10615–10618`.

- [x] Selected457055 copies its native effect0 fraction of actual off-mark direct, periodic and white damage to the caster's live457129 mark through457236. Damage to the marked victim does not self-copy. Preserve the source's resolved crit/modifiers without rolling them again or recursively dispatching residual callbacks. Source: `sc_rogue.cpp6618–6631,11568–11605`.

### Shared auto-attack speed

- [x] Owned381988 and315496 add live attack haste, scaled by381988 effect1, to SnD's native auto-attack-speed bonus. Preserve ordinary haste, unrelated attack-time modifiers, aura amounts, duration, extension, GCD/cast clocks and ranged timing. Assassination and Outlaw use the same path. Source: `sc_rogue.cpp9140–9148,11280–11282`.

### Assassination target damage

- [ ] Selected381640 dynamically multiplies qualifying native direct and periodic damage by its native effect0 percentage per owned ticking bleed/Deadly Poison and active poison debuff, once per effect rather than stack. Exclude Kingsbane from the count and Envenom from the bonus; preserve the sourced Internal Bleeding/Mutilated Flesh exceptions. Use Potent Assassin's native effect0/1 family masks. Source: `sc_rogue.cpp1718–1732,2347–2357,2489–2499,4263,6188,6281–6287,9085–9090`.
- [ ] Selected381798 dynamically multiplies native damage below its strict native effect1 health threshold by effect0. Use the native mastery family masks and labels rather than school-wide guesses. Source: `sc_rogue.cpp1718–1725,2347–2350,2489–2492`.

## How it works

- [Native aura runtime](../wiki/systems/auras.md).
- [Spell casting and impact context](../wiki/systems/spell-casting.md).
- [Derived character statistics](../wiki/systems/character-stats.md).

## Implementation inventory

- `crates/server-combat/src/auras/{rogue,scripts}.rs` — native Rogue state and stealth lifecycle.
- `crates/server-combat/src/spell_cast/scripts/{rogue_children,rogue_hero}.rs` — sourced impact/periodic children and mark lifecycle.
- `crates/server-combat/src/{combat.rs,spell_cast/mod.rs,spell_cast/scripts.rs}` — swing and cast dispatch integration.
- `crates/server-combat/src/spell_cast/scripts/{cast_relationships,policies}.rs` — native recovery windows and captured target/resource context.

## Tests asserting this spec

Paths below are under `crates/server-simulation/src/dungeon_simulator/dps_parity/`:

- `assassination_doomblade_tests.rs` — one real Mutilate's two hands fund the complete native bleed and its own Lethal Dose bonus.
- `assassination_kingsbane_tests.rs` — actual direct/periodic damage with and without native Mastery/Implacable passives proves the pinned double-dip ratios.
- `assassination_focus_tests.rs` — actual off-mark direct/periodic damage copies to the owned mark at the native fraction, with no self-copy or second crit.
- `assassination_white_tests.rs` — actual landed weapon hits and marked/unmarked retargeting.
- `assassination_target_tests.rs` — real poison recipient state/health damage and Kingsbane count exclusion.
- `assassination_poison_mechanics_tests.rs` — earned ramp and Amplifying consumption.
- `assassination_hero_tests.rs` — real stealth exit/spread, native children and mark cycles.
- `assassination_seal_fate_tests.rs` — real builder crits and Poisoned Knife exclusion.
- `rogue_swift_slasher_tests.rs` — real259/260 swing clocks.
- `rogue_finisher_tests.rs` — unchanged Outlaw SnD start/extension regression.

## Known gaps (current cycle)

- [ ] Qualify these mechanics and spec259 ST/3T output against the parent contract's ±10% band.
- [ ] Quietus Celeris, mark-target death, Unshakeable Drive's delayed consumer, and Mass Casualty remain unqualified by this slice.

## Out of scope

- Numerical stat/coefficient substitutions; native data remains authoritative.
- SnD duration/extension changes, owned by the Outlaw lane.
- Other Rogue-spec mechanics beyond the explicitly shared coating and attack-speed paths.
