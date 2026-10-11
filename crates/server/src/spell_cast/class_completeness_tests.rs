//! Shared, source-derived class effects against native Retail rows.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Health;

const PIERCING_CHILL: u32 = 377359;
const CHILL_STREAK: u32 = 204167;
const FROSTBOLT: u32 = 116;

fn fixture() -> Harness {
    let data = world_db().expect("real world.db");
    let spells = [PIERCING_CHILL, CHILL_STREAK, FROSTBOLT]
        .into_iter()
        .map(|id| {
            let mut spell = data.store.get(id).expect("native spell row").clone();
            for effect in &mut spell.effects {
                effect.variance = 0.0;
            }
            spell
        })
        .collect();
    harness(spells, &[], 6).expect("real world.db")
}

fn apply_chill(h: &mut Harness, caster: Entity) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target: h.npc,
            spell_id: PIERCING_CHILL,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
}

fn hit(h: &mut Harness, spell_id: u32) -> f32 {
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 5000.0;
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    super::execute_cast(h.app.world_mut(), h.caster, &spell, Some(h.npc)).unwrap();
    h.advance(10); // Native Chill Streak has a 20 yd/s missile.
    5000.0 - h.health(h.npc)
}

#[test]
fn class_completeness_native_label_vulnerability_increases_owned_chill_streak() {
    let mut h = fixture();
    let base = hit(&mut h, CHILL_STREAK);
    assert!(base > 0.0);
    let caster = h.caster;
    apply_chill(&mut h, caster);
    let amplified = hit(&mut h, CHILL_STREAK);
    assert_eq!(amplified, (base * 1.12).round());
}

#[test]
fn class_completeness_native_label_vulnerability_excludes_other_spell_and_caster() {
    let mut h = fixture();
    let frostbolt = hit(&mut h, FROSTBOLT);
    let chill = hit(&mut h, CHILL_STREAK);
    let other = spawn_player(&mut h.app, 6, 0.0);
    apply_chill(&mut h, other);
    assert_eq!(hit(&mut h, CHILL_STREAK), chill);
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.npc,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == PIERCING_CHILL,
    );
    let caster = h.caster;
    apply_chill(&mut h, caster);
    assert_eq!(hit(&mut h, FROSTBOLT), frostbolt);
}

#[test]
fn class_completeness_native_label_vulnerability_expires() {
    let mut h = fixture();
    let base = hit(&mut h, CHILL_STREAK);
    let caster = h.caster;
    apply_chill(&mut h, caster);
    h.advance(200);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.npc,
        PIERCING_CHILL
    ));
    assert_eq!(hit(&mut h, CHILL_STREAK), base);
}
