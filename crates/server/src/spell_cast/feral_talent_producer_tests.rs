//! SimC76f8118 sc_druid.cpp3957–4047,4277–4326,4884–4997,12276–12403.
use super::{CAT_FORM, Harness, Inbox, SHRED, fixture, spawn_npc, spawn_player};
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const RAKE: u32 = 1822;
const RAKE_BLEED: u32 = 155722;
const THRIVING_GROWTH: u32 = 439528;
const BLOODSEEKER_VINES: u32 = 439531;
const FEROCIOUS_BITE: u32 = 22568;
const UNSEEN_PREDATOR: u32 = 1263657;
const UNSEEN_SLASH: u32 = 1263890;
const UNSEEN_SWIPE: u32 = 1263908;
const UNSEEN_BLEED: u32 = 1271863;

fn cast_melee(h: &mut Harness, spell: u32) {
    let npc = h.npc;
    h.cast_and_deliver(spell, Some(npc));
    h.stop_attack();
    h.advance(35);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == spell));
    assert!(h.health(npc) > 0.0, "native recipient must remain alive");
}

fn regenerate_energy(h: &mut Harness) {
    for _ in 0..240 {
        let (current, maximum) =
            crate::spell_cast::powers::power_value(h.app.world(), h.caster, PowerType::Energy)
                .unwrap();
        if current >= maximum {
            return;
        }
        h.advance(1);
    }
    panic!("native Energy regeneration did not refill its actual pool");
}

fn landed_five_point_bite(h: &mut Harness) -> bool {
    for _ in 0..12 {
        if h.power(PowerType::ComboPoints) == 5.0 {
            break;
        }
        regenerate_energy(h);
        cast_melee(h, SHRED);
    }
    assert_eq!(h.power(PowerType::ComboPoints), 5.0);
    regenerate_energy(h);
    let energy = h.power(PowerType::Energy);
    let before = h.inbox.log.len();
    cast_melee(h, FEROCIOUS_BITE);
    let landed = h.inbox.log[before..].iter().any(|event| {
        event.spell_id == Some(FEROCIOUS_BITE)
            && event.kind == CombatLogKind::Damage
            && event.amount > 0
    });
    if landed {
        assert_eq!(h.power(PowerType::ComboPoints), 0.0);
        assert!(h.power(PowerType::Energy) < energy);
    }
    landed
}

fn has_periodic_damage(h: &Harness, spell: u32) -> bool {
    h.inbox.log.iter().any(|event| {
        event.spell_id == Some(spell)
            && event.kind == CombatLogKind::Damage
            && event.periodic
            && event.amount > 0
    })
}

fn has_unseen_damage(h: &Harness) -> bool {
    h.inbox.log.iter().any(|event| {
        matches!(
            event.spell_id,
            Some(UNSEEN_SLASH | UNSEEN_SWIPE | UNSEEN_BLEED)
        ) && event.kind == CombatLogKind::Damage
            && event.amount > 0
    })
}

#[test]
#[ignore = "RED: eligible native Rake ticks must accumulate Thriving Growth"]
fn feral_producers_vines_native_rake_produces_owned_vines_and_retains_them_after_unlearn() {
    let mut h = fixture(&[RAKE, THRIVING_GROWTH]);
    let other = spawn_npc(&mut h.app, 4.0);
    let ally = spawn_player(&mut h.app, 11, 4.0);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        THRIVING_GROWTH
    ));
    'casts: for _ in 0..40 {
        cast_melee(&mut h, RAKE);
        for _ in 0..260 {
            if crate::auras::has_aura(h.app.world(), h.npc, BLOODSEEKER_VINES) {
                break 'casts;
            }
            h.advance(1);
        }
    }
    assert!(
        has_periodic_damage(&h, RAKE_BLEED),
        "native Rake periodic damage is a producer prerequisite"
    );
    let vines = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == BLOODSEEKER_VINES)
        .expect("native Thriving Growth must produce an actual owned Vines application");
    assert_eq!(vines.caster, h.caster);
    for unit in [h.caster, other, ally] {
        assert!(!crate::auras::has_aura(
            h.app.world(),
            unit,
            BLOODSEEKER_VINES
        ));
    }
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            CAT_FORM, SHRED, RAKE,
        ]));
    h.advance(3);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        THRIVING_GROWTH
    ));
    h.inbox = Inbox::default();
    h.advance(180);
    assert!(has_periodic_damage(&h, BLOODSEEKER_VINES));
    assert!(
        h.inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(BLOODSEEKER_VINES) && event.kind == CombatLogKind::Damage
            })
            .all(|event| {
                event.source == Some(h.caster.to_bits()) && event.target == Some(h.npc.to_bits())
            })
    );
    assert!(!h.inbox.log.iter().any(|event| {
        event.spell_id == Some(BLOODSEEKER_VINES)
            && matches!(
                event.kind,
                CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
            )
    }));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.npc,
        BLOODSEEKER_VINES
    ));
}

#[test]
fn feral_producers_vines_native_rake_without_talent_never_uses_catalog_driver() {
    let mut h = fixture(&[RAKE]);
    for _ in 0..8 {
        cast_melee(&mut h, RAKE);
        h.advance(300);
        if has_periodic_damage(&h, RAKE_BLEED) {
            break;
        }
    }
    assert!(has_periodic_damage(&h, RAKE_BLEED));
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| { event.spell_id == Some(BLOODSEEKER_VINES) })
    );
}

#[test]
#[ignore = "RED: native spent combo points and talent chance must produce Unseen attacks"]
fn feral_producers_unseen_native_five_point_bites_produce_owned_slash_and_bleed() {
    let mut h = fixture(&[FEROCIOUS_BITE, UNSEEN_PREDATOR]);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        UNSEEN_PREDATOR
    ));
    let mut landed = 0;
    for _ in 0..16 {
        if landed_five_point_bite(&mut h) {
            landed += 1;
        }
        if has_unseen_damage(&h) {
            break;
        }
    }
    assert!(
        landed > 0,
        "actual native five-point Bite damage qualifies the producer"
    );
    assert!(
        has_unseen_damage(&h),
        "native15% per spent point must produce an actual Unseen attack"
    );
    h.advance(180);
    assert!(h.inbox.log.iter().any(|event| {
        event.spell_id == Some(UNSEEN_SLASH)
            && event.kind == CombatLogKind::Damage
            && event.amount > 0
    }));
    assert!(has_periodic_damage(&h, UNSEEN_BLEED));
    assert!(
        h.inbox
            .log
            .iter()
            .filter(|event| {
                matches!(
                    event.spell_id,
                    Some(UNSEEN_SLASH | UNSEEN_SWIPE | UNSEEN_BLEED)
                ) && event.kind == CombatLogKind::Damage
            })
            .all(|event| {
                event.source == Some(h.caster.to_bits()) && event.target == Some(h.npc.to_bits())
            })
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(UNSEEN_SWIPE))
    );
}

#[test]
fn feral_producers_unseen_native_five_point_bite_without_talent_has_no_children() {
    let mut h = fixture(&[FEROCIOUS_BITE]);
    let landed = (0..8).any(|_| landed_five_point_bite(&mut h));
    assert!(
        landed,
        "native Bite must actually damage after spending earned points"
    );
    h.advance(180);
    assert!(!has_unseen_damage(&h));
}
