# Cast while casting

Native `SPELL_ATTR4_ALLOW_CAST_WHILE_CASTING` spells can execute during an existing cast. Source: TC a352b1fa `Spell.cpp530–531`; actual70448 Fire Blast108853. See [spell casting](../wiki/systems/spell-casting.md#cast-while-casting--2026-10-21).

## What it must do

- [ ] Honor the attribute for creature casts, player requests and follower kit admission.
- [ ] Execute instant Fire Blast before the original Fireball finishes, including requests inside the spell queue window.
- [ ] Preserve the original cast's duration, progress and eventual damage.
- [ ] Keep ordinary second casts blocked and preserve native costs, GCD, cooldown and charge checks.

## How it works

- [Native admission and queue handling](../wiki/systems/spell-casting.md#cast-while-casting--2026-10-21).

## Implementation inventory

- `crates/server/src/spell_cast/mod.rs`: attribute predicate, admission and player queue delay.
- `crates/server/src/followers/kit_ai.rs`: shared native predicate before kit casts.

## Tests asserting this spec

- `crates/server/src/followers/cast_while_casting_tests.rs`: actual108853/133 creature, kit and player early/queue-window execution; ordinary cast rejection.
- `crates/server/src/spell_cast/queue_tests.rs`: retained ordinary queue/resource/cooldown controls.

## Known gaps (current cycle)

- [ ] Enabled native GREEN and retained control proof pending.

## Out of scope

Rotation priority and action selection remain owned by the rotation lane; this changes native eligibility only.
