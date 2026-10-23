//! Actual learning, timed cast, source actor expiry and existing DismissPet RPC.
use super::*;
use crate::spell_cast::summon::SummonedCreature;
use shared::components::{ModelDisplay, Npc, UnitSummonedBy};

struct Balloon {
    toy: u32,
    spell: u32,
    entry: u32,
    display: u32,
}

fn actors(h: &mut Harness, balloon: &Balloon) -> Vec<Entity> {
    let owner = h.caster;
    let world = h.app.world_mut();
    world
        .query::<(Entity, &Npc, &SummonedCreature)>()
        .iter(world)
        .filter(|(_, npc, summon)| {
            npc.template_id == balloon.entry
                && summon.owner == owner
                && summon.spell_id == balloon.spell
        })
        .map(|(entity, _, _)| entity)
        .collect()
}

fn start_cast(h: &mut Harness, balloon: &Balloon) {
    let results = activate_item(h, balloon.toy);
    assert!(
        results
            .iter()
            .any(|r| r.operation == ToyOperation::Use && r.error.is_none()),
        "{results:?}"
    );
    assert!(
        actors(h, balloon).is_empty(),
        "source1s cast must not spawn during first350ms"
    );
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == balloon.spell));
    advance_journal(h, 22);
}

fn assert_actor(h: &mut Harness, balloon: &Balloon) -> Entity {
    let actors = actors(h, balloon);
    assert_eq!(actors.len(), 1);
    let actor = actors[0];
    let world = h.app.world();
    assert_eq!(
        world.get::<ModelDisplay>(actor).unwrap().display_id,
        balloon.display
    );
    assert_eq!(
        world.get::<UnitSummonedBy>(actor).unwrap().0,
        h.caster.to_bits()
    );
    assert_eq!(world.get::<InMap>(actor), world.get::<InMap>(h.caster));
    assert_eq!(
        crate::spell_cast::summon::current_critter(world, h.caster),
        Some(actor)
    );
    assert!(
        world
            .get::<SummonedCreature>(actor)
            .unwrap()
            .expires_at
            .is_some()
    );
    assert!(world.get::<UnitFlags>(actor).unwrap().0 & UnitFlags::NON_ATTACKABLE_2 != 0);
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == balloon.spell && !cd.is_gcd && cd.duration_ms > 0)
    );
    assert!(h.inbox.go.iter().any(|go| go.spell_id == balloon.spell));
    actor
}

fn use_again(h: &mut Harness, balloon: &Balloon) -> Entity {
    h.inbox.go.clear();
    start_cast(h, balloon);
    assert_actor(h, balloon)
}

fn summon_replacing_journal(
    h: &mut Harness,
    yellow_balloon: u64,
    character: u64,
    balloon: &Balloon,
) -> Entity {
    let journal_actor = summon_journal_actor(h, yellow_balloon, YELLOW_BALLOON);
    learn_item(h, character, balloon.toy);
    h.select(Some(h.npc));
    advance_journal(h, 2);
    start_cast(h, balloon);
    let first = assert_actor(h, balloon);
    assert!(
        h.app.world().get_entity(journal_actor).is_err(),
        "same authored slot6 replacement"
    );
    let connection = h.connection;
    let replies = take_journal_replies(h, connection);
    assert!(replies.iter().any(|r| r.summoned_pet_id.is_none()));
    first
}

fn assert_natural_expiry(h: &mut Harness, first: Entity) {
    advance_journal(h, 1_100);
    assert!(
        h.app.world().get_entity(first).is_ok(),
        "still before source60s expiry"
    );
    advance_journal(h, 120);
    assert!(h.app.world().get_entity(first).is_err(), "source60s expiry");
    assert_eq!(
        crate::spell_cast::summon::current_critter(h.app.world(), h.caster),
        None
    );
}

fn dismiss_reuse_and_leave_map(h: &mut Harness, balloon: &Balloon) {
    let second = use_again(h, balloon);
    let replies = request_dismiss(h);
    assert!(replies.iter().any(|r| r.error.is_none()));
    assert!(h.app.world().get_entity(second).is_err());
    let third = use_again(h, balloon);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(InMap::continent(1));
    advance_journal(h, 2);
    assert!(h.app.world().get_entity(third).is_err());
}

fn prove_balloon(balloon: Balloon) {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, ids, character) = fixture(&dir.path().join("balloon.redb"));
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&crate::ground::ground_data_dir().join("los"))
            .unwrap(),
    );
    let health = *h.app.world().get::<Health>(h.caster).unwrap();
    let position = *h.app.world().get::<Position>(h.caster).unwrap();
    let first = summon_replacing_journal(&mut h, ids.yellow_balloon, character, &balloon);
    assert_natural_expiry(&mut h, first);
    dismiss_reuse_and_leave_map(&mut h, &balloon);
    assert_eq!(h.app.world().get::<Health>(h.caster), Some(&health));
    assert_eq!(h.app.world().get::<Position>(h.caster), Some(&position));
    let db = &h.app.world().resource::<GameDatabase>().db;
    assert!(toy_store::load(db, 1).contains_key(&balloon.toy));
    assert_eq!(
        load_journal(db, 1)
            .get(ids.yellow_balloon)
            .unwrap()
            .species_id,
        YELLOW_BALLOON
    );
}

#[test]
#[ignore = "caster-offset toy qualification; run explicitly"]
fn toy_coverage_caster_offset_yellow_balloon_learning_cast_source_expiry_dismiss_and_reuse() {
    prove_balloon(Balloon {
        toy: 75042,
        spell: 105229,
        entry: 56083,
        display: 38341,
    });
}

#[test]
#[ignore = "caster-offset toy qualification; run explicitly"]
fn toy_coverage_caster_offset_brewers_balloon_learning_cast_source_expiry_dismiss_and_reuse() {
    prove_balloon(Balloon {
        toy: 245946,
        spell: 1240088,
        entry: 246042,
        display: 129910,
    });
}
