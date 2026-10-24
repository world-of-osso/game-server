//! TC Spell.cpp3900–3918/4212–4226: actual CAST/HIT/FINISH queue inputs.
use server::auras::procs::{AuraEvent, PendingAuraEvents};
use server::spell_cast::test_harness::{Harness, harness};
use shared::components::PowerType;

const SHIELD: u32 = 17;
const FORTITUDE: u32 = 21562;
const HELPFUL_SPELL: u32 = 0x4000;
const CAST_SUCCESSFUL: u32 = 4;

struct NativeCast {
    events: Vec<AuraEvent>,
    mana_spent: f32,
}

fn native_self_cast(spell: u32) -> NativeCast {
    let mut h = harness(Vec::new(), &[spell], 5).unwrap();
    h.advance(3);
    let before = current_mana(&h);
    h.app
        .world_mut()
        .resource_mut::<PendingAuraEvents>()
        .0
        .clear();
    server::spell_cast::creature_cast(h.app.world_mut(), h.caster, spell, Some(h.caster), false)
        .unwrap();
    assert!(
        server::auras::has_aura(h.app.world(), h.caster, spell),
        "ordinary native buff{spell} really applied"
    );
    NativeCast {
        events: read_spell_events(&h, spell),
        mana_spent: before - current_mana(&h),
    }
}

fn current_mana(h: &Harness) -> f32 {
    server::spell_cast::powers::power_value(h.app.world(), h.caster, PowerType::Mana)
        .expect("native primary Mana exists")
        .0
}

fn read_spell_events(h: &Harness, spell: u32) -> Vec<AuraEvent> {
    let events: Vec<_> = h
        .app
        .world()
        .resource::<PendingAuraEvents>()
        .0
        .iter()
        .filter(|event| event.spell_id == Some(spell))
        .cloned()
        .collect();
    assert!(
        !events.is_empty(),
        "actual native buff{spell} emitted combat events"
    );
    assert!(events.iter().all(|event| event.actor == h.caster));
    events
}

fn cast_event(events: &[AuraEvent]) -> &AuraEvent {
    events
        .iter()
        .find(|event| event.actor_mask[1] & CAST_SUCCESSFUL != 0)
        .expect("native successful ordinary cast event")
}

fn hit_event(events: &[AuraEvent]) -> &AuraEvent {
    events
        .iter()
        .find(|event| event.target == Some(event.actor))
        .expect("native buff actually landed on its caster")
}

#[test]
#[ignore = "native full-row fixture; retain ordinary shield and event behavior"]
fn native_proc_event_context_ordinary_shield_control() {
    let _seed = server::random::seed_scope(70448);
    let native = native_self_cast(SHIELD);
    // Bare harness has no required specialization-cost aura: all four native17 rows are inapplicable.
    assert_eq!(native.mana_spent, 0.0);
    assert_eq!(cast_event(&native.events).target, None);
    assert_eq!(hit_event(&native.events).actor_mask, [HELPFUL_SPELL, 0]);
}

#[test]
#[ignore = "native TC CAST combines low class flags and successful-cast flag"]
fn native_proc_event_context_cast_carries_class_flags_and_native_context() {
    let _seed = server::random::seed_scope(70448);
    let native = native_self_cast(SHIELD);
    let event = cast_event(&native.events);
    assert_eq!(event.actor_mask, [HELPFUL_SPELL, CAST_SUCCESSFUL]);
    assert_eq!(event.target_mask, [0; 2]);
    assert_eq!(event.target, None);
    let context = event
        .context
        .as_ref()
        .expect("actual CAST context, not inferred defaults");
    assert_eq!(context.spell_phase_mask, 1);
    assert_eq!(context.spell_type_mask, 7);
    assert_eq!(context.hit_mask, 1);
    assert_eq!(context.school_mask, 2);
}

#[test]
#[ignore = "native TC landed no-damage spell has actual HIT context"]
fn native_proc_event_context_hit_carries_school_type_and_normal_outcome() {
    let _seed = server::random::seed_scope(70448);
    let native = native_self_cast(SHIELD);
    let event = hit_event(&native.events);
    let context = event
        .context
        .as_ref()
        .expect("actual HIT context, not damage-total inference");
    assert_eq!(context.spell_phase_mask, 2);
    assert_eq!(context.spell_type_mask, 4);
    assert_eq!(context.hit_mask, 1);
    assert_eq!(context.school_mask, 2);
    assert_eq!(context.attack_type, None);
}

#[test]
#[ignore = "native TC FINISH is once per spell, actor-only, after landed effects"]
fn native_proc_event_context_finish_is_one_actor_only_aggregate() {
    let _seed = server::random::seed_scope(70448);
    let native = native_self_cast(SHIELD);
    let finished: Vec<_> = native
        .events
        .iter()
        .filter(|event| event.target.is_none() && event.actor_mask[1] == 0)
        .collect();
    assert_eq!(
        finished.len(),
        1,
        "one actual native FINISH event, not zero or one per effect"
    );
    let event = finished[0];
    assert_eq!(event.actor_mask, [HELPFUL_SPELL, 0]);
    assert_eq!(event.target_mask, [0; 2]);
    let context = event
        .context
        .as_ref()
        .expect("actual aggregate FINISH context");
    assert_eq!(context.spell_phase_mask, 4);
    assert_eq!(context.spell_type_mask, 4);
    assert_eq!(context.hit_mask, 1);
    assert_eq!(context.school_mask, 2);
}

#[test]
#[ignore = "native Fortitude unconditional1% mana row captures a positive prepared cost"]
fn native_proc_event_context_positive_cost_and_item_mod_identity_are_captured() {
    let _seed = server::random::seed_scope(70448);
    let native = native_self_cast(FORTITUDE);
    assert!(
        native.mana_spent > 0.0,
        "actual unconditional native Fortitude cost was consumed"
    );
    let context = cast_event(&native.events)
        .context
        .as_ref()
        .expect("cast's actual prepared cost receipt");
    assert!(context.has_power_cost);
    assert!(!context.cast_from_item);
    assert!(context.applied_aura_modifiers.is_empty());
    assert_eq!(context.attack_type, None);
}

#[test]
#[ignore = "native Shield conditional spec costs are not applicable in this bare harness"]
fn native_proc_event_context_inapplicable_power_rows_do_not_invent_cost() {
    let _seed = server::random::seed_scope(70448);
    let native = native_self_cast(SHIELD);
    assert_eq!(native.mana_spent, 0.0);
    let context = cast_event(&native.events)
        .context
        .as_ref()
        .expect("cast's actual zero-cost receipt");
    assert!(!context.has_power_cost);
}
