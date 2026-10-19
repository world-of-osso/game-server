use super::*;
use crate::spell_cast::effects::EffectCtx;
use crate::spell_cast::powers::{PowerPools, power_value};
use shared::components::{ModelDisplay, Npc, Position, UnitLevel};

#[test]
#[ignore = "RED: ordinary BG completion stops owner combat but not its summoned pet"]
fn bg_end_pets_hunter_living_owner_stops_owned_pet_without_reset() {
    assert_owner_pet_stopped(3, false);
}

#[test]
#[ignore = "RED: ordinary BG completion stops owner combat but not its summoned pet"]
fn bg_end_pets_warlock_living_owner_stops_owned_pet_without_reset() {
    assert_owner_pet_stopped(9, false);
}

#[test]
#[ignore = "RED: ordinary BG completion stops owner combat but not its summoned pet"]
fn bg_end_pets_warlock_dead_owner_resurrects_then_stops_owned_pet_without_reset() {
    assert_owner_pet_stopped(9, true);
}

fn install_pet_inputs(h: &mut Harness) {
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    h.app.insert_resource(data.store.clone());
    h.app
        .insert_resource(crate::spell_cast::test_harness::power_table(data));
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::hunter_pet::read_pet_catalog(&db).unwrap());
    h.app
        .insert_resource(crate::pet_control::load_pet_levelup_spells(&db).unwrap());
}

fn create_native_pet(h: &mut Harness, owner: &Traveler, class: u8) -> Entity {
    if h.app
        .world()
        .get::<shared::components::UnitFactionTemplate>(owner.player)
        .is_none()
    {
        h.app
            .world_mut()
            .entity_mut(owner.player)
            .insert(shared::components::UnitFactionTemplate(
                crate::faction::faction_template_for_race(1),
            ));
    }
    h.app
        .world_mut()
        .get_mut::<Player>(owner.player)
        .unwrap()
        .class = class;
    let spell_id = if class == 3 { 13481 } else { 688 };
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    let context = EffectCtx {
        caster: owner.player,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    if class == 3 {
        let position = *h.app.world().get::<Position>(owner.player).unwrap();
        let map = h.map_of(owner).unwrap();
        let target = h
            .app
            .world_mut()
            .spawn((
                Npc {
                    template_id: 299,
                    name: "Diseased Young Wolf".into(),
                },
                ModelDisplay { display_id: 1413 },
                position,
                map,
                UnitLevel(10),
                Health {
                    current: 100.0,
                    max: 100.0,
                },
            ))
            .id();
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.effect == crate::hunter_pet::EFFECT_TAME_CREATURE)
            .unwrap();
        crate::hunter_pet::effect_tame_creature(h.app.world_mut(), &context, effect, target);
        assert_eq!(h.app.world().get::<Health>(target).unwrap().current, 0.0);
        assert!(
            h.app
                .world()
                .get::<crate::combat::DeathState>(target)
                .is_some(),
            "native tame despawns original spawn without deleting its respawn entity"
        );
    } else {
        let effect = spell
            .effects
            .iter()
            .find(|effect| effect.effect == crate::hunter_pet::EFFECT_SUMMON_PET)
            .unwrap();
        assert_eq!(effect.misc_values[0], 416);
        crate::hunter_pet::effect_summon_pet(h.app.world_mut(), &context, effect);
    }
    let pet = crate::hunter_pet::summoned_pet(h.app.world(), owner.player)
        .expect("native tame/summon producer installs current owned pet");
    assert_eq!(
        h.app
            .world()
            .get::<crate::hunter_pet::HunterPet>(pet)
            .unwrap()
            .owner,
        owner.player
    );
    let maximum = h.app.world().get::<Health>(pet).unwrap().max;
    h.app.world_mut().get_mut::<Health>(pet).unwrap().current = maximum * 0.6;
    pet
}

fn start_pet_fight(h: &mut Harness, pet: Entity, target: Entity) {
    let world = h.app.world_mut();
    let mut commands = world.commands();
    crate::combat::start_attack(&mut commands, pet, target, None);
    world.flush();
    crate::combat::deal_direct_damage(world, pet, target, 0.01);
    assert!(world.get::<crate::combat::AutoAttack>(pet).is_some());
    assert_eq!(world.get::<CombatStatus>(pet), Some(&CombatStatus(true)));
}

fn spawn_unrostered_opponent(h: &mut Harness, id: u64, map: InMap) -> Traveler {
    let who = h.spawn_at_level(id, 25, map, STOCKADE_ENTRANCE_TRIGGER);
    h.app
        .world_mut()
        .get_mut::<Health>(who.player)
        .unwrap()
        .current = 10000.0;
    who
}

fn drive_pet_fights_to_end(fixture: &mut CleanupFixture, fights: &[(Entity, Entity)]) {
    for _ in 0..200 {
        for &(pet, target) in fights {
            crate::combat::deal_direct_damage(fixture.entered.h.app.world_mut(), pet, target, 0.01);
            assert_eq!(
                fixture.entered.h.app.world().get::<CombatStatus>(pet),
                Some(&CombatStatus(true))
            );
        }
        fixture.entered.h.receive_pvp_updates(1);
        if battleground(&fixture.entered.h, &fixture.entered.players[0]).phase == BgPhase::Ended {
            return;
        }
    }
    panic!("native node objective did not reach terminal victory");
}

fn assert_owner_pet_stopped(class: u8, dead_owner: bool) {
    let mut fixture = node_fixture();
    let owner = &fixture.entered.players[1];
    install_pet_inputs(&mut fixture.entered.h);
    let pet = create_native_pet(&mut fixture.entered.h, owner, class);
    let target = spawn_unrostered_opponent(&mut fixture.entered.h, 2000, fixture.entered.copy);
    start_pet_fight(&mut fixture.entered.h, pet, target.player);
    let owner_entity = owner.player;
    if dead_owner {
        let killer_index = fixture
            .entered
            .teams
            .iter()
            .position(|team| *team != fixture.entered.teams[1])
            .unwrap();
        let killer = fixture.entered.players[killer_index].player;
        deal_lethal_damage(&mut fixture.entered.h, killer, owner);
    }
    let health = fixture
        .entered
        .h
        .app
        .world()
        .get::<Health>(pet)
        .unwrap()
        .current;
    let primary = fixture
        .entered
        .h
        .app
        .world()
        .get::<PowerPools>(pet)
        .unwrap()
        .order[0];
    let power = power_value(fixture.entered.h.app.world(), pet, primary).unwrap();
    drive_pet_fights_to_end(&mut fixture, &[(pet, target.player)]);
    let world = fixture.entered.h.app.world();
    assert_eq!(
        world.get::<CombatStatus>(pet),
        Some(&CombatStatus(false)),
        "completed owner's actively summoned pet must leave combat before result"
    );
    assert!(world.get::<crate::combat::AutoAttack>(pet).is_none());
    assert_eq!(
        crate::hunter_pet::summoned_pet(world, owner_entity),
        Some(pet)
    );
    assert_eq!(world.get::<Health>(pet).unwrap().current, health);
    assert_eq!(power_value(world, pet, primary).unwrap(), power);
    assert!(world.get::<Health>(owner_entity).unwrap().current > 0.0);
    assert_eq!(
        world.get::<CombatStatus>(owner_entity),
        Some(&CombatStatus(false))
    );
    fixture.entered.h.advance(5);
    assert_eq!(
        fixture.entered.h.app.world().get::<CombatStatus>(pet),
        Some(&CombatStatus(false))
    );
    assert_eq!(
        fixture
            .entered
            .h
            .app
            .world()
            .get::<Health>(pet)
            .unwrap()
            .current,
        health
    );
    assert_eq!(
        crate::hunter_pet::summoned_pet(fixture.entered.h.app.world(), owner_entity),
        Some(pet)
    );
}

#[test]
fn bg_end_pets_departed_owner_pet_remains_in_combat() {
    let mut fixture = node_fixture();
    install_pet_inputs(&mut fixture.entered.h);
    let owner = fixture.entered.players.last().unwrap();
    let pet = create_native_pet(&mut fixture.entered.h, owner, 9);
    let target = spawn_unrostered_opponent(&mut fixture.entered.h, 2030, fixture.entered.copy);
    start_pet_fight(&mut fixture.entered.h, pet, target.player);
    let h = &mut fixture.entered.h;
    h.app
        .world_mut()
        .entity_mut(owner.connection)
        .insert(pair::<LeaveBattleground>());
    h.app
        .world_mut()
        .get_mut::<MessageSender<LeaveBattleground>>(owner.connection)
        .unwrap()
        .send::<PvpChannel>(LeaveBattleground {
            match_id: fixture.entered.match_id,
        });
    let updates = h.receive_pvp_updates(5);
    assert!(
        updates[&owner.connection]
            .iter()
            .all(|update| update.error.is_none()),
        "native owner departure: {updates:?}"
    );
    assert_eq!(h.new_worlds(owner).len(), 1);
    h.ack(owner);
    assert_eq!(h.map_of(owner), Some(STORMWIND));
    assert_eq!(
        crate::hunter_pet::summoned_pet(h.app.world(), owner.player),
        Some(pet)
    );
    drive_pet_fights_to_end(&mut fixture, &[(pet, target.player)]);
    assert_eq!(
        fixture.entered.h.app.world().get::<CombatStatus>(pet),
        Some(&CombatStatus(true))
    );
    assert!(
        fixture
            .entered
            .h
            .app
            .world()
            .get::<crate::combat::AutoAttack>(pet)
            .is_some()
    );
}

#[test]
fn bg_end_pets_other_copy_and_unentered_owner_pets_remain_in_combat() {
    let mut fixture = node_fixture();
    install_pet_inputs(&mut fixture.entered.h);
    let outsider = fixture
        .entered
        .h
        .spawn_at_level(2001, 25, STORMWIND, STOCKADE_ENTRANCE_TRIGGER);
    let mut fights = Vec::new();
    for (index, owner) in [&fixture.other_players[0], &outsider]
        .into_iter()
        .enumerate()
    {
        let pet = create_native_pet(&mut fixture.entered.h, owner, 9);
        let target = spawn_unrostered_opponent(
            &mut fixture.entered.h,
            2010 + index as u64,
            if index == 0 {
                fixture.other_copy
            } else {
                STORMWIND
            },
        );
        start_pet_fight(&mut fixture.entered.h, pet, target.player);
        fights.push((pet, target.player));
    }
    drive_pet_fights_to_end(&mut fixture, &fights);
    for (pet, _) in fights {
        assert_eq!(
            fixture.entered.h.app.world().get::<CombatStatus>(pet),
            Some(&CombatStatus(true))
        );
        assert!(
            fixture
                .entered
                .h
                .app
                .world()
                .get::<crate::combat::AutoAttack>(pet)
                .is_some()
        );
    }
}
