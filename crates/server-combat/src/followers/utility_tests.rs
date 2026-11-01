//! Native enemy-cast test fixtures.
use shared::casting::CastState;
use shared::components::Position;

pub(super) fn start_enemy_fireball(
    h: &mut crate::spell_cast::test_harness::Harness,
    unit: bevy::prelude::Entity,
) {
    let enemy = h.npc;
    prepare_enemy_spec(h, enemy, 63);
    let at = *h.app.world().get::<Position>(enemy).unwrap();
    h.app.world_mut().entity_mut(unit).insert(Position {
        x: at.x - 2.0,
        ..at
    });
    crate::spell_cast::creature_cast(h.app.world_mut(), enemy, 133, Some(unit), false).unwrap();
    assert!(h.app.world().get::<CastState>(enemy).is_some());
}

pub(super) fn prepare_enemy_spec(
    h: &mut crate::spell_cast::test_harness::Harness,
    enemy: bevy::prelude::Entity,
    spec: u32,
) {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let mage = super::catalog::load_catalog(&db)
        .unwrap()
        .profiles
        .into_iter()
        .find(|profile| profile.spec_id == spec)
        .unwrap();
    let (stats, pools, passives) = super::spawning::class_stats(h.app.world(), &mage, 12).unwrap();
    let mana = shared::components::Mana {
        current: stats.base_mana,
        max: stats.base_mana,
    };
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert((stats, pools, mana, shared::components::Level(12)));
    crate::auras::sync_passives(
        h.app.world_mut(),
        enemy,
        crate::auras::AuraSource::KnownSpell,
        &passives,
    );
}

pub(super) fn protect_enemy_cast_with_native_interrupt_immunity(
    h: &mut crate::spell_cast::test_harness::Harness,
    target: bevy::prelude::Entity,
) {
    const UNENDING_RESOLVE: u32 = 104773;
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(UNENDING_RESOLVE)
        .unwrap();
    assert!(
        spell
            .effects
            .iter()
            .any(|effect| effect.aura == 77 && effect.misc_values[0] == 26)
    );
    let effect_mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: target,
            target,
            spell_id: UNENDING_RESOLVE,
            effect_mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(crate::auras::has_aura(
        h.app.world(),
        target,
        UNENDING_RESOLVE
    ));
}
