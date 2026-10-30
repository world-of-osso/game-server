//! TC Unit.cpp10611–10670 gathers live candidates at event time, before later applications.
//! Native Leydrinker tests aura application/cast ordering, not talent availability.
use crate::auras::{ActiveAuras, has_aura};
use crate::spell_cast::test_harness::{ObservedCombatEvents, harness};
use shared::protocol::CombatLogKind;

const LEYDRINKER: u32 = 453758;
const FIREBALL: u32 = 133;

#[test]
fn generated_weapon_crit_proc_spends_charge_only_on_critical_hit() {
    use crate::auras::{AuraApplication, AuraSource, procs};

    let mut h = harness(vec![], &[66889], 8).unwrap();
    h.advance(3);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 66889,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    let emit = |h: &mut crate::spell_cast::test_harness::Harness, hit_mask| {
        procs::push_event(
            h.app.world_mut(),
            procs::AuraEvent {
                actor: h.caster,
                target: Some(h.npc),
                action_target: None,
                actor_mask: [procs::PROC_DEAL_MELEE_SWING, 0],
                target_mask: [0; 2],
                spell_id: None,
                can_proc: true,
                cast_used_proc_modifier: false,
                periodic: false,
                damage: 100.0,
                heal: None,
                hit_mask,
                reflected: false,
            },
        );
        h.advance(3);
    };
    emit(&mut h, procs::PROC_HIT_NORMAL);
    assert!(has_aura(h.app.world(), h.caster, 66889));
    emit(&mut h, procs::PROC_HIT_CRITICAL);
    assert!(!has_aura(h.app.world(), h.caster, 66889));
}

#[test]
fn native_proc_event_candidates_exclude_later_ley_drinker_but_consume_on_next_cast() {
    let _seed = crate::random::seed_scope(70448);
    let mut h = harness(vec![], &[LEYDRINKER, FIREBALL], 8).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    assert!(!has_aura(h.app.world(), h.caster, LEYDRINKER));
    h.cast_and_deliver(LEYDRINKER, Some(h.caster));
    assert!(h.inbox.go.iter().any(|cast| cast.spell_id == LEYDRINKER));
    assert!(
        h.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.spell_id == Some(LEYDRINKER) && event.kind == CombatLogKind::AuraApplied
            }),
        "actual native self cast must apply the aura after raising its cast-success event"
    );
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == LEYDRINKER)
        .expect("an aura applied after its cast event cannot spend a charge on that older event");
    assert_eq!(
        aura.charges, 1,
        "native charge remains available for a subsequent event"
    );
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    h.advance(100);
    assert!(h.inbox.go.iter().any(|cast| cast.spell_id == FIREBALL));
    assert!(
        !has_aura(h.app.world(), h.caster, LEYDRINKER),
        "an aura already present when the next real cast event is raised must still spend its native charge"
    );
}
