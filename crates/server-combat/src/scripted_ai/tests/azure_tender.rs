//! TC a352b1fa boss_leymor.cpp:191164 npc_arcane_tender and its spell scripts.
//! The player is refilled between steps: this proves ability timing and native
//! effects, not party survivability.
use super::*;

const LEYMOR: u32 = 186644;
const TENDER: u32 = 191164;
const STASIS_RITUAL: u32 = 375732;
const WILD_ERUPTION: u32 = 375652;
const WILD_ERUPTION_FIELD: u32 = 375647;
const ERRATIC_GROWTH: u32 = 375596;
const ERRATIC_GROWTH_TRANSFORM: u32 = 375602;

fn azure_tenders() -> (Harness, Entity, Vec<Entity>) {
    let (mut h, _, map) =
        encounter_fixture(LEYMOR, 2515, 0, 205, None, EncounterState::NotStarted, true);
    h.advance(0.1);
    let in_copy = |h: &mut Harness, entry| -> Vec<Entity> {
        h.creatures_of(entry)
            .into_iter()
            .filter(|&actor| h.app.world().get::<InMap>(actor) == Some(&map))
            .collect()
    };
    let leymor = in_copy(&mut h, LEYMOR)[0];
    let tenders = in_copy(&mut h, TENDER);
    assert_eq!(tenders.len(), 3, "three authored Arcane Tenders");
    (h, leymor, tenders)
}

fn stasis_casters(h: &Harness, leymor: Entity) -> Vec<Entity> {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(leymor)
        .unwrap()
        .iter()
        .filter(|aura| aura.spell_id == STASIS_RITUAL)
        .map(|aura| aura.caster)
        .collect()
}

fn advance_refilled(h: &mut Harness, seconds: f32) {
    let player = h.player();
    for _ in 0..(seconds * 10.0).round() as usize {
        h.advance(0.1);
        let mut health = h.app.world_mut().get_mut::<Health>(player).unwrap();
        health.current = health.max;
    }
}

#[test]
fn azure_tender_channels_native_stasis_ritual_on_leymor_until_engaged() {
    let (mut h, leymor, tenders) = azure_tenders();
    let mut casters = stasis_casters(&h, leymor);
    casters.sort();
    let mut expected = tenders.clone();
    expected.sort();
    assert_eq!(casters, expected, "each Tender channels375732 on Leymor");
    let tender = tenders[0];
    let position = *h.app.world().get::<Position>(tender).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(position);
    h.engage(tender);
    advance_refilled(&mut h, 0.5);
    assert!(
        !stasis_casters(&h, leymor).contains(&tender),
        "native EnteringCombat channel interrupt ends the engaged Tender's ritual"
    );
}

#[test]
fn azure_tender_wild_eruption_and_erratic_growth_follow_source_clocks() {
    let (mut h, _, tenders) = azure_tenders();
    let tender = tenders[0];
    let position = *h.app.world().get::<Position>(tender).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(position);
    h.engage(tender);
    advance_refilled(&mut h, 11.8);
    assert_eq!(h.casts(0, tender, WILD_ERUPTION), 0);
    advance_refilled(&mut h, 2.0);
    assert_eq!(
        h.casts(0, tender, WILD_ERUPTION),
        1,
        "source12s Wild Eruption completes its native1.5s cast"
    );
    advance_refilled(&mut h, 1.0);
    let fields = h
        .app
        .world_mut()
        .query::<&crate::spell_cast::area_trigger::SpellAreaTrigger>()
        .iter(h.app.world())
        .filter(|field| field.caster == tender && field.spell_id == WILD_ERUPTION_FIELD)
        .count();
    assert_eq!(
        fields, 1,
        "one player target sends one375650 missile whose375647 creates the field"
    );
    assert!(!h.has_aura(player, ERRATIC_GROWTH));
    advance_refilled(&mut h, 7.5);
    assert!(
        h.has_aura(player, ERRATIC_GROWTH),
        "source22s Erratic Growth places its aura on one enemy"
    );
    assert!(!h.has_aura(player, ERRATIC_GROWTH_TRANSFORM));
    advance_refilled(&mut h, 3.8);
    assert!(!h.has_aura(player, ERRATIC_GROWTH));
    assert!(
        h.has_aura(player, ERRATIC_GROWTH_TRANSFORM),
        "aura495 expiry casts375602 from the Tender on its target"
    );
}
