# Rain of Fire ordinary-client visibility

Lord Overheat's actual Rain of Fire86636 dynamic object is visible through ordinary same-copy interest, mirroring clients' visibility of the caster's dynamic object. Geometry follows active70448 SpellEffect88312/SpellRadius8; the harness policy lives in the [engine contract](../../../game-engine/docs/specs/bot-harness.md).

## What it must do

- [ ] Publish actual caster/spell/radius and resolved Position only for86636 persistent areas.
- [ ] Use existing same-copy/range visibility; other copies never receive the area.
- [ ] Remove on actual server expiry or caster despawn; no predicted client expiry.
- [ ] Preserve actual area application, periodic damage and removal when a unit leaves.
- [ ] Keep unit-targeted Overheat86633 distinct from ground geometry.

## How it works

- [Spell casting](../wiki/systems/spell-casting.md)
- [Shared geometry contract](../../../shared-protocol/docs/specs/dynamic-objects.md)

## Implementation inventory

- `crates/server/src/spell_cast/area_aura.rs`: actual area spawn/interest/expiry/damage lifetime.

## Tests asserting this spec

- `crates/server/src/scripted_ai/tests/rainfire_visibility.rs`: actual boss cast, source radius/duration/destination, same-copy interest, movement/damage boundary, expiry/caster removal.
- `crates/server/src/scripted_ai/tests.rs`: existing real Rain of Fire/Overheat spell effects.

## Known gaps (current cycle)

- [ ] Joint protocol/server/engine consumer gate and fresh native ordinary-input escape proof.

## Out of scope

- Other areas, geometry/stat/kit changes, follower behavior, guessed cast-centre or lifetime reconstruction.
