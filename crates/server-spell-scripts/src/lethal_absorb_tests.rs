//! Native lethal-hit and heal-back behavior for the two class talents.
use crate::combat::{
    auras::{self, AuraApplication, AuraSource},
    spell_cast::{self, effects::EffectCtx},
};
use crate::test_support::{Harness, harness, world_db};
use shared::components::Health;

const HIT: u32 = 314616;

fn fixture(class: u8, talent: u32, spells: &[u32]) -> Harness {
    let mut ids = vec![HIT, talent, 431483];
    ids.extend_from_slice(spells);
    let mut h = harness(Vec::new(), &ids, class).unwrap();
    let unit = h.caster;
    auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: unit,
            target: unit,
            spell_id: talent,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert!(auras::has_aura(h.app.world(), unit, talent));
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 200.0;
    h
}

fn lethal_hit(h: &mut Harness) {
    spell_cast::creature_cast(h.app.world_mut(), h.npc, HIT, Some(h.caster), false).unwrap();
    h.advance(10);
}

#[test]
fn purgatory_rescues_a_lethal_hit_absorbs_healing_and_locks_out_the_next() {
    let mut h = fixture(6, 114556, &[116888, 123981, 123982]);
    let unit = h.caster;
    lethal_hit(&mut h);
    assert_eq!(h.health(unit), 1.0);
    assert!(auras::has_aura(h.app.world(), unit, 116888));
    assert!(auras::has_aura(h.app.world(), unit, 123981));
    let spell = world_db()
        .unwrap()
        .spell_store()
        .get(431483)
        .unwrap()
        .clone();
    let ctx = EffectCtx {
        caster: unit,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    spell_cast::effects::apply_fixed_heal(h.app.world_mut(), &ctx, unit, 10000.0);
    assert!(!auras::has_aura(h.app.world(), unit, 116888));
    assert!(h.health(unit) > 1.0);
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 200.0;
    lethal_hit(&mut h);
    assert_eq!(h.health(unit), 0.0);
}

#[test]
fn purgatory_unhealed_shroud_kills_on_native_expiry() {
    let mut h = fixture(6, 114556, &[116888, 123981, 123982]);
    lethal_hit(&mut h);
    assert_eq!(h.health(h.caster), 1.0);
    h.advance(70);
    assert_eq!(h.health(h.caster), 0.0);
}

#[test]
fn infinite_health_fixture_does_not_trigger_lethal_rescues() {
    for (class, talent, lockout, spells) in [
        (6, 114556, 123981, vec![116888, 123981, 123982]),
        (12, 209258, 209261, vec![209261, 187827]),
    ] {
        let mut h = fixture(class, talent, &spells);
        let unit = h.caster;
        h.app
            .world_mut()
            .entity_mut(unit)
            .insert(crate::combat::test_support::SimcInfiniteHealth);
        h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 1.0;
        lethal_hit(&mut h);
        assert_eq!(h.health(unit), 1.0);
        assert!(!auras::has_aura(h.app.world(), unit, lockout));
        let hit = h
            .inbox
            .log
            .iter()
            .find(|event| {
                event.spell_id == Some(HIT) && event.kind == shared::protocol::CombatLogKind::Damage
            })
            .expect("native incoming damage");
        assert_eq!(hit.absorbed, 0);
    }
}

#[test]
fn last_resort_transforms_once_then_the_next_lethal_hit_kills() {
    let mut h = fixture(12, 209258, &[209261, 187827]);
    let unit = h.caster;
    lethal_hit(&mut h);
    assert!(h.health(unit) > 0.0);
    assert!(auras::has_aura(h.app.world(), unit, 209261));
    assert!(auras::has_aura(h.app.world(), unit, 187827));
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 200.0;
    lethal_hit(&mut h);
    assert_eq!(h.health(unit), 0.0);
}
