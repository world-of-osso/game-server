//! Native Unit form power selection: real NPCs retain identity and real burn behavior.
use bevy::prelude::*;
use shared::components::{Player, PowerType};

use super::powers::{PowerPools, power_value};
use super::test_harness::*;

const CAT: u32 = 768;
const MOONKIN: u32 = 197625;
const BITE: u32 = 22568;
const RALIA: u32 = 232800;
const CRENNA: u32 = 209072;

fn native_npc(entry: u32) -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[CAT, MOONKIN, BITE], 11).unwrap();
    h.app.insert_resource(load_native_creature_world(&[entry]));
    let source = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&source).unwrap());
    let actor = crate::followers::native_fixture::spawn(h.app.world_mut(), h.caster, entry)
        .expect("authentic NPC profile/template and production initializer");
    assert!(h.app.world().get::<Player>(actor).is_none());
    assert_eq!(
        h.app
            .world()
            .get::<crate::followers::Follower>(actor)
            .unwrap()
            .profile
            .entry,
        entry
    );
    (h, actor)
}

#[test]
fn native_npc_cat_form_exposes_energy_without_player_identity() {
    let (h, actor) = native_npc(RALIA);
    assert!(
        crate::auras::has_aura(h.app.world(), actor, CAT),
        "production Ralia initializer casts her authored Cat Form"
    );
    let pools = h.app.world().get::<PowerPools>(actor).unwrap();
    assert!(pools.get(PowerType::Energy).is_some());
    assert!(pools.get(PowerType::ComboPoints).is_some());
    assert_eq!(pools.order.first(), Some(&PowerType::Energy));
}

#[test]
fn native_npc_cat_real_bite_power_burn_spends_energy() {
    let (mut h, actor) = native_npc(RALIA);
    assert!(crate::auras::has_aura(h.app.world(), actor, CAT));
    let spell = world_db().unwrap().spell_store().get(BITE).unwrap().clone();
    let index = spell
        .effects
        .iter()
        .position(|effect| effect.index == 1)
        .unwrap();
    let cap = spell.effects[index].base_points;
    let before = power_value(h.app.world(), actor, PowerType::Energy)
        .unwrap()
        .0;
    assert!(
        cap > 0.0 && before >= cap,
        "real native pool must cover the authored burn cap"
    );
    // Same native effect dispatch as the retained power-burn controls. No cast-cost
    // bypass is claimed: this isolates the actual22568 effect1 burn, not a full finisher.
    let plan = vec![super::PlannedEffect {
        index,
        units: vec![actor],
        dest: None,
        chain_multipliers: Vec::new(),
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }];
    super::resolve_effects(h.app.world_mut(), actor, &spell, &plan, 0);
    let after = power_value(h.app.world(), actor, PowerType::Energy)
        .unwrap()
        .0;
    assert!(
        (before - after - cap).abs() < 0.01,
        "native NPC Cat Bite burn: before{before}, after{after}, authored cap{cap}"
    );
    assert!(h.app.world().get::<Player>(actor).is_none());
}

#[test]
fn native_player_cat_form_keeps_energy_selection() {
    let mut h = harness(Vec::new(), &[CAT], 11).unwrap();
    h.cast_and_deliver(CAT, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), h.caster, CAT));
    assert!(h.app.world().get::<Player>(h.caster).is_some());
    assert_eq!(
        h.app
            .world()
            .get::<PowerPools>(h.caster)
            .unwrap()
            .order
            .first(),
        Some(&PowerType::Energy)
    );
}

#[test]
fn native_npc_moonkin_caster_form_keeps_mana() {
    // Crenna starts in caster form; Ralia transitions out of her native Cat form.
    for entry in [CRENNA, RALIA] {
        let (mut h, actor) = native_npc(entry);
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id: MOONKIN,
                effect_mask: 1 << 1,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert!(crate::auras::has_aura(h.app.world(), actor, MOONKIN));
        assert!(h.app.world().get::<Player>(actor).is_none());
        assert_eq!(
            h.app
                .world()
                .get::<PowerPools>(actor)
                .unwrap()
                .order
                .first(),
            Some(&PowerType::Mana)
        );
    }
}
