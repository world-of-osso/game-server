//! Spirit Bomb consumes active native fragments and launches its delayed damage.
use super::combat;
use bevy::prelude::*;
use combat::soul_fragments::{self, FragmentFilter};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

struct PendingBomb {
    at: f64,
    target: Option<Entity>,
    multiplier: f32,
}
#[derive(Component, Default)]
struct PendingBombs(Vec<PendingBomb>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeLaunch(launch),
    SpellScriptHook::AfterNativeCastUpdate(update),
];

fn launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 247454 {
        return;
    }
    let percent = spell_cast::scripts::script_effect_value(world, caster, 247454, 0);
    let maximum = spell_cast::scripts::script_effect_value(world, caster, 247454, 1);
    assert!(
        maximum >= 0.0 && maximum.fract() == 0.0,
        "invalid Spirit Bomb fragment cap"
    );
    let consumed =
        soul_fragments::consume_active(world, caster, FragmentFilter::Any, maximum as usize, true)
            .expect("native Spirit Bomb fragment consumption");
    if consumed == 0 {
        return;
    }
    super::vengeance_soul_cleave::trigger_untethered_rage(world, caster, consumed);
    // SimC76f8118 sc_demon_hunter.cpp5371–5390: snapshot the fragment-scaled
    // damage and deliver its native child one second after consumption.
    let pending = PendingBomb {
        at: spell_cast::now(world) + 1.0,
        target: spell.cast_primary_target,
        multiplier: 1.0 + percent * consumed as f32 / 100.0,
    };
    world
        .entity_mut(caster)
        .entry::<PendingBombs>()
        .or_default()
        .into_mut()
        .0
        .push(pending);
}

fn update(world: &mut World) {
    let now = spell_cast::now(world);
    let casters: Vec<_> = world
        .query_filtered::<Entity, With<PendingBombs>>()
        .iter(world)
        .collect();
    for caster in casters {
        let due = {
            let mut pending = world
                .get_mut::<PendingBombs>(caster)
                .expect("Spirit Bomb queue");
            let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut pending.0)
                .into_iter()
                .partition(|bomb| bomb.at <= now);
            pending.0 = later;
            due
        };
        for bomb in due {
            if !spell_cast::cast_triggered_with_overrides(
                world,
                caster,
                247455,
                bomb.target,
                1,
                spell_cast::TriggeredSpellOverrides {
                    damage_multiplier: Some(bomb.multiplier),
                    ..Default::default()
                },
            ) {
                error!(?caster, "native Spirit Bomb damage refused");
            }
        }
    }
}
