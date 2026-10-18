# Health aura-state admission qualification

Source qualification for Execute163201/state2 and Massacre281000/state13. Runtime basis: server258e28b6/master97b9882b, protocolb701f32, native12.1.5.70391; primary reference TCa352b1fa. **No health-state implementation or runtime acceptance follows from this investigation.** The raw fields load, but the server has no unit aura-state store/producer or state admission consumer yet.

## Qualified source contract

### Integer thresholds

`Unit.h:800–809` stores health/max as `uint64`; below/above comparisons are strict. `Util.h:71–75` calculates `T(base * static_cast<float>(pct) / 100.0f)`: float multiply/divide followed by integer truncation, not an untruncated percentage comparison.

| State | Source condition |
|---|---|
| 2 | Below20 |
| 6 | Below25 |
| 13 | Below35 |
| 21 | Below20 or Above80 |
| 23 | Above75 |
| 24 | Below35 or Above80 |
| 25 | Below50 |

At max999, state2's integer threshold is199 (198 passes,199 fails); state13's is349 (348 passes,349 fails). At max5000, state13 admits1749 and refuses1750. An explorer's contrary1750-admitted example was rejected. Its later200.5/1001 example also used an incorrectly untruncated200.2 threshold; the source threshold is200 and both original/projected current values fail the strict below test.

Runtime `Health` remains f32. Player max-health computation floors and current rescaling truncates (`stats.rs:385–390,648–665`), disproving a broad claim that stats necessarily retain fractions. Special writes can retain fractions: ghost restoration (`death.rs:816`) and Ardent Defender (`auras/mod.rs:1111`) are confirmed examples. Integer input cases are qualified; fractional histories and f32 precision beyond2^24 do not certify whole-health-system parity or justify a new budget policy.

### State is published by unit updates, not health setters

`Unit::Update` processes spell events/current spells, combat/reactives, then the seven health conditions in the table's order, only while alive (`Unit.cpp:430–504`). `SetHealth` calls health-change aura468 handling, **not** these state updates (`10039–10068`). `ClearAllReactives` clears states1/7 only (`10929–10938`); the inspected death transition does not directly clear the seven health flags.

A changed bit runs immediate side effects (`6096–6149`): gain triggers matching enabled known passives; loss removes self-cast auras requiring that state, including non-passives for these health flags. Learn-time gating reads the raw state without spell/caster arguments (`Player.cpp:3119–3120`), unlike cast-time `HasAuraState` and its aura262 matching.

### Packet, queued-cast and target ordering

In-world CMSG_CAST_SPELL is THREADSAFE and uses `MapSessionFilter` (`Opcodes.cpp:287`, `WorldSession.cpp:63–104`). `Map::Update` processes sessions before each player's update and nearby-cell traversal (`Map.cpp:663–720`). A queued player request runs after that player's `Unit::Update` (`Player.cpp:945–950`). Thus immediate requests see the prior unit-update snapshot; queued requests can encounter different target freshness depending on actor/cell traversal. Publishing health state at every health write or always using the freshest health is not the source contract.

Current Bevy0.19 runs FixedMainLoop before Update. Client request/instant casts execute in Update; completion/queued starts execute in FixedUpdate, and the aura chain follows casts/autoattacks. A source-compatible snapshot boundary and passive timing need an explicit integration design and behavioral proof; adding a global polling predicate alone does not establish equivalence.

## Authenticated inputs and remaining seams

Native rows confirm281001E0 aura332 amount281000/misc163201. Both roots have classSet4 and mask0=537133312, matching52437E0 aura262 mask536870912. Earlier missing-override/mask claims were rejected. State2/13 are global bits; only14/16 are per-caster. Aura262 makes `HasAuraState` return true, so it permits a required check but makes an excluded-state check fail. Vehicle-kit and charmer/owner exemptions apply only to state checks, not aura-spell/type references (`SpellInfo.cpp:2497–2517`). Mounted/riding, lazy occupied-seat state and credit/lifetime ownership are not established substitutes for those identities.

The native receipt contains14 health-state-required spells and five passives:53497/169616 require13;210632/373456/375542 require23. None of these five directly changes health. There is no state2 passive in this snapshot. Their skill-line memberships are empty; current trait/spec/pet knowledge membership remains to be authenticated, not invented for legal-build acceptance.

Rust passive sync ignores these state fields. Its `KnownSpell`-source cleanup can conflict with a triggered state's `Cast`-source aura, and direct requeueing would miss non-passive self-aura removal and cast-level side effects. A new disabled-spell field was not established as necessary: existing active knowledge iteration already excludes replacement grants. Pet passive-book and follower resync gaps remain distinct; they are not permission to fabricate spellbooks or expand the health feature into a pet/stat rewrite.

## Implementation still open

- Prove immediate, queued and completion snapshot semantics against actual runtime ordering; no on-demand float shortcut or assumed actor traversal.
- Integrate passive learn/gain/loss and source ownership/unlearn cleanup without incompatible direct-health filtering.
- Qualify vehicle-kit and direct charmer/owner mappings from actual producers.
- Bound any integer-view/precision claims without altering health, resource or clock budgets.

The preserved full-trigger helper target bypass is also an **open flag gap**, not native compatibility: `Spell.cpp:3488–3492` only overrides BAD_TARGETS for IGNORE_TARGET_CHECK, while target aura errors remain separate. Prior ordinary-field slices deliberately did not implement the full flag contract; their bypass control must not be promoted to native target-state acceptance.

## Sources

- TCa352b1fa `Unit.h:800–809`, `Util.h:71–75`, `SharedDefines.h:2830–2864`, `Unit.cpp:430–504,6096–6186,9178–9230,10039–10090,10929–10938` — numeric/state/lifecycle contract.
- Same pin `Opcodes.cpp:287`, `WorldSession.cpp:63–104`, `Map.cpp:663–825`, `Player.cpp:945–950,3100–3120`, `Spell.cpp:3488–3492,3759,5873–5888,5957–5963`, `SpellInfo.cpp:2497–2517` — processing phases and checks. Newly cached primary files have URL/SHA256 receipts under evidence D's `tc-a352b1fa/`.
- [Spell casting](../systems/spell-casting.md), [auras](../systems/auras.md), [spell metadata](../systems/spell-info.md) — current runtime and separate implemented aura-spell consumers.
- `shared-protocol-s2/src/components.rs:134–137`, cached Bevy0.19 `main_schedule.rs:224–231`, server `stats.rs`, `auras/mod.rs`, `spell_cast/mod.rs` — literal runtime fields/schedules.
- Evidence D=`data/class-completeness6-2026-10-12`: `health-aura-state-qualification-{goal,result}.json`, `health-state-native-passives.json`; native restriction equality and payloads are retained by the prior importer/loader/Execute receipts.

## See Also

- [Class completeness contract](../../specs/class-completeness.md)
- [Native aura-spell admission](../systems/spell-casting.md#native-required-caster-aura-spell-admission--2026-10-16)
