//! Dire Command summons the actual native Dire Beast from successful Kill Commands.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::spell_cast::{self, TriggeredSpellOverrides, script_registry::SpellScriptHook};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] =
    &[SpellScriptHook::AfterElementalLaunch(after_command)];
const DIRE_COMMAND: u32 = 378743;
const DIRE_BEAST: u32 = 219199;

#[derive(Component, Default)]
struct Attempts(u32);

fn roll_command(world: &mut World, owner: Entity) -> bool {
    let probability = f64::from(spell_cast::scripts::script_effect_value(
        world,
        owner,
        DIRE_COMMAND,
        0,
    )) / 100.0;
    let coefficient = combat::random::accumulated_proc_constant(probability);
    let attempts = world.get::<Attempts>(owner).map_or(1, |state| state.0 + 1);
    let success = combat::random::random::<f64>() < coefficient * f64::from(attempts);
    world
        .entity_mut(owner)
        .insert(Attempts(if success { 0 } else { attempts }));
    success
}

fn read_lifetime(world: &World, owner: Entity) -> i32 {
    let base = world
        .resource::<SpellStore>()
        .get(DIRE_BEAST)
        .expect("native Dire Beast")
        .misc
        .as_ref()
        .and_then(|misc| misc.duration.as_ref())
        .expect("native Dire Beast duration")
        .duration_ms;
    let haste = world
        .get::<combat::stats::CharacterStats>(owner)
        .expect("native Hunter stats")
        .haste_pct;
    let swing = 2000.0 / (1.0 + f64::from(haste) / 100.0);
    let attacks = f64::from(base) / swing;
    let count = attacks.floor() + f64::from(combat::random::random::<f64>() < attacks.fract());
    (count * swing).round() as i32
}

fn after_command(world: &mut World, owner: Entity, spell: &SpellInfo, foreground: bool) {
    if !foreground || spell.id != 34026 || !has_talent(world, owner, DIRE_COMMAND) {
        return;
    }
    let Some(primary) = spell.cast_primary_target else {
        return;
    };
    // SimC76f8118 hunter6610–6613/8396: successful KC rolls the selected
    // native chance using accumulated PRD. hunter1736–1755 converts the native
    // summon lifetime into a haste-scaled, probabilistic integer swing count.
    if roll_command(world, owner) {
        let duration = read_lifetime(world, owner);
        let values = TriggeredSpellOverrides {
            duration_ms: Some(duration),
            ..Default::default()
        };
        if !spell_cast::cast_triggered_with_overrides(
            world,
            owner,
            DIRE_BEAST,
            Some(primary),
            1,
            values,
        ) {
            warn!(?owner, ?primary, "Native Dire Command summon refused");
        }
    }
}
