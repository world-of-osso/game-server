//! TC core Coyote removal uses source-owned target conditions, not any nearby summon.
use super::*;
use crate::spell_cast::summon::SummonedCreature;

fn install_coyote_sources(h: &mut Harness) {
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    h.app
        .insert_resource(crate::spell_cast::summon::load_summon_properties(&conn).unwrap());
    h.app
        .insert_resource(crate::conditions::load_spell_target_conditions(&path).unwrap());
}

fn summon_source_coyote(h: &mut Harness, owner: Entity) -> Entity {
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        owner,
        51149,
        None,
        0
    ));
    h.advance(2);
    current_source_coyote(h, owner)
}

fn current_source_coyote(h: &mut Harness, owner: Entity) -> Entity {
    let world = h.app.world_mut();
    world
        .query::<(Entity, &SummonedCreature)>()
        .iter(world)
        .find(|(_, summon)| summon.owner == owner && summon.spell_id == 51149)
        .map(|(entity, _)| entity)
        .expect("actual source51149 Coyote")
}

#[test]
fn toy_coyote_owned_condition_selects_only_source_owner_not_other_summoner() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, _) = setup(&dir.path().join("coyote-conditions.redb"));
    install_coyote_sources(&mut h);
    let owner = h.caster;
    let other = spawn_player(&mut h.app, 8, 5.0);
    let own = summon_source_coyote(&mut h, owner);
    let other_owned = summon_source_coyote(&mut h, other);
    // Ordinary TempSummon's summoner does not establish source GetOwnerGUID.
    let unowned = crate::zones::summon_creature(h.app.world_mut(), owner, 28267, [0.0; 4]).unwrap();
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(52173)
        .unwrap()
        .clone();
    let selected = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        owner,
        None,
        &spell,
        &spell.effects[0],
        0,
        false,
    )
    .unwrap();
    assert_eq!(selected, vec![own]);
    assert!(!selected.contains(&other_owned));
    assert!(!selected.contains(&unowned));
    // Same entry and owner in another world copy remains outside the source search.
    h.app
        .world_mut()
        .entity_mut(own)
        .insert(crate::maps::InMap {
            map_id: 0,
            instance_id: 42,
        });
    let selected = crate::spell_cast::targets::select_targets(
        h.app.world_mut(),
        owner,
        None,
        &spell,
        &spell.effects[0],
        0,
        false,
    )
    .unwrap();
    assert!(selected.is_empty());
}

#[test]
fn toy_coyote_hat_real_use_cancel_casts_source_child_and_only_despawns_owned_coyote() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("hat-lifecycle.redb"));
    install_coyote_sources(&mut h);
    h.app.init_resource::<ObservedTriggeredCasts>();
    let owner = h.caster;
    let other = spawn_player(&mut h.app, 8, 5.0);
    let other_owned = summon_source_coyote(&mut h, other);
    let unowned = crate::zones::summon_creature(h.app.world_mut(), owner, 28267, [0.0; 4]).unwrap();
    learn_effect_toy(&mut h, character, 134019);
    h.select(Some(other));
    h.advance(2);
    let health = *h
        .app
        .world()
        .get::<shared::components::Health>(owner)
        .unwrap();
    let position = *h.app.world().get::<Position>(owner).unwrap();
    assert_eq!(activate(&mut h, 134019)[0].error, None);
    h.advance(10);
    let coyote = current_source_coyote(&mut h, owner);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(coyote)
            .unwrap()
            .template_id,
        28267
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::ModelDisplay>(coyote)
            .unwrap()
            .display_id,
        25265
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::UnitSummonedBy>(coyote)
            .unwrap()
            .0,
        owner.to_bits()
    );
    assert!(
        h.app
            .world()
            .get::<crate::creature_ai::FollowOwner>(coyote)
            .is_some()
    );
    assert!(
        h.app
            .world()
            .get::<SummonedCreature>(coyote)
            .unwrap()
            .expires_at
            .is_none()
    );
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(owner)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 52172)
        .unwrap();
    assert_eq!(aura.effects[0].amount, 52173.0);
    assert!(aura.expires_at.is_none());
    assert!(
        h.app
            .world()
            .get::<shared::components::UnitAuras>(owner)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == 52172 && aura.caster == Some(owner.to_bits()))
    );
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == 52172 && cd.category == 1476 && cd.duration_ms == 3000)
    );
    assert_eq!(
        activate(&mut h, 134019)[0].error.as_deref(),
        Some("OnCooldown")
    );
    h.advance(40);
    assert!(h.app.world().get_entity(coyote).is_ok());
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: 52172 });
    h.advance(5);
    assert!(!crate::auras::has_aura(h.app.world(), owner, 52172));
    assert!(
        h.app
            .world()
            .resource::<ObservedTriggeredCasts>()
            .0
            .iter()
            .any(|&(caster, spell, target)| caster == owner && spell == 52173 && target.is_none())
    );
    assert!(h.app.world().get_entity(coyote).is_err());
    assert!(h.app.world().get_entity(other_owned).is_ok());
    assert!(h.app.world().get_entity(unowned).is_ok());
    assert_eq!(
        h.app.world().get::<shared::components::Health>(owner),
        Some(&health)
    );
    assert_eq!(h.app.world().get::<Position>(owner), Some(&position));
    h.advance(70); // Natural source category1476 recovery, no clock clearing.
    assert_eq!(activate(&mut h, 134019)[0].error, None);
    h.advance(10);
    let replacement = current_source_coyote(&mut h, owner);
    assert_ne!(replacement, coyote);
}

#[test]
fn toy_coyote_hat_missing_removal_child_changed_amount_or_conditions_refuses_before_use() {
    for drift in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("hat-drift.redb"));
        install_coyote_sources(&mut h);
        learn_effect_toy(&mut h, character, 134019);
        let mut spells: Vec<_> = [52172, 51149, 52173]
            .into_iter()
            .filter(|&spell| drift != 0 || spell != 52173)
            .map(|spell| {
                h.app
                    .world()
                    .resource::<crate::spell_info::SpellStore>()
                    .get(spell)
                    .unwrap()
                    .clone()
            })
            .collect();
        if drift == 1 {
            spells.iter_mut().find(|s| s.id == 52172).unwrap().effects[0].base_points = 52174.0;
        }
        if drift == 2 {
            h.app
                .insert_resource(crate::conditions::SpellTargetConditions::default());
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(spells));
        assert!(activate(&mut h, 134019)[0].error.is_some());
        assert!(!crate::auras::has_aura(h.app.world(), h.caster, 52172));
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == 52172));
        assert!(!h.inbox.cooldowns.iter().any(|cd| cd.spell_id == 52172));
        let world = h.app.world_mut();
        assert!(
            world
                .query::<&SummonedCreature>()
                .iter(world)
                .all(|summon| summon.owner != h.caster)
        );
    }
}

#[test]
fn toy_coyote_hat_census_admits_only_source_root_not_unrelated_dummy_contracts() {
    let report = report_toy_census(&crate::test_data::world_db_path(), "TOY_COYOTE_REPORT");
    let rows = report["toys"].as_array().unwrap();
    let hat = rows.iter().find(|r| r["item_id"] == 134019).unwrap();
    assert!(hat["unavailable"].is_null(), "{hat}");
    for item in [38301, 18986, 166662] {
        let row = rows.iter().find(|r| r["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}
