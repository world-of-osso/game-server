//! Target57 selects the actual raid member, not any ally or a substituted caster.
use super::*;
use crate::group::{GroupRegistry, OnlineMember, OnlineView};
use crate::persistence::ClientId;

const SPOTLIGHT: u32 = 230924;
const SPOTLIGHT_SPELL: u32 = 467730;
const BLESSING_OF_PROTECTION: u32 = 1022;
const POWER_INFUSION: u32 = 10060;

fn spawn_spotlight_friend(h: &mut Harness, character: u64, x: f32) -> Entity {
    let player = spawn_player(&mut h.app, 8, x);
    h.app
        .world_mut()
        .entity_mut(player)
        .insert((ClientId(character), CombatStatus(false)));
    player
}

fn invite_spotlight_friend(h: &mut Harness, character: u64, recipient: Entity) {
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ClientId(character));
    let recipient_id = h.app.world().get::<ClientId>(recipient).unwrap().0;
    let online = OnlineView::from_iter(
        [
            (character, h.caster, "SpotOwner"),
            (recipient_id, recipient, "SpotFriend"),
        ]
        .into_iter()
        .map(|(id, entity, name)| (id, spotlight_online_member(h, entity, name))),
    );
    let mut groups = h.app.world_mut().resource_mut::<GroupRegistry>();
    groups.invite(character, "SpotFriend", &online);
    groups.respond_invite(recipient_id, true);
    assert!(groups.is_in_same_group(character, recipient_id));
}

fn spotlight_online_member(h: &Harness, entity: Entity, name: &str) -> OnlineMember {
    OnlineMember {
        entity: entity.to_bits(),
        name: name.into(),
        class: h
            .app
            .world()
            .get::<shared::components::Player>(entity)
            .unwrap()
            .class,
        level: h.app.world().get::<Level>(entity).unwrap().0,
        portrait: default(),
        dungeon_difficulty: 1,
        raid_difficulty: default(),
    }
}

#[test]
fn toy_coverage_spotlight_native_target57_rejects_foreign_ally_before_cast() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("raid57.redb"));
    install_visual_source(&mut h);
    let foreign = spawn_spotlight_friend(&mut h, character + 1, 2.0);
    let owner = h.caster;
    let result = crate::spell_cast::creature_cast(
        h.app.world_mut(),
        owner,
        BLESSING_OF_PROTECTION,
        Some(foreign),
        false,
    );
    assert!(
        matches!(result, Err(crate::spell_cast::CastFailure::InvalidTarget)),
        "{result:?}"
    );
    h.advance(5);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        owner,
        BLESSING_OF_PROTECTION
    ));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        foreign,
        BLESSING_OF_PROTECTION
    ));
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.spell_id == BLESSING_OF_PROTECTION)
    );
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == BLESSING_OF_PROTECTION)
    );
}

#[test]
fn toy_coverage_spotlight_native_target21_still_accepts_foreign_ally() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("ally21.redb"));
    install_visual_source(&mut h);
    let foreign = spawn_spotlight_friend(&mut h, character + 1, 2.0);
    let owner = h.caster;
    let result = crate::spell_cast::creature_cast(
        h.app.world_mut(),
        owner,
        POWER_INFUSION,
        Some(foreign),
        false,
    );
    assert!(result.is_ok(), "{result:?}");
    h.advance(5);
    assert!(crate::auras::has_aura(
        h.app.world(),
        foreign,
        POWER_INFUSION
    ));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        owner,
        POWER_INFUSION
    ));
}

#[test]
fn toy_coverage_spotlight_actual_group_recipient_replication_cancel_and_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("spotlight-recipient.redb"));
    install_visual_source(&mut h);
    let recipient = spawn_spotlight_friend(&mut h, character + 1, 2.0);
    let foreign = spawn_spotlight_friend(&mut h, character + 2, 3.0);
    invite_spotlight_friend(&mut h, character, recipient);
    let recipient_connection = spawn_peer_connection(&mut h.app, recipient, 2);
    h.app
        .world_mut()
        .entity_mut(recipient_connection)
        .insert(InWorld {
            character_id: character + 1,
        });
    learn_effect_toy(&mut h, character, SPOTLIGHT);
    h.select(Some(recipient));
    h.advance(3);
    let owner = h.caster;
    let health = *h.app.world().get::<Health>(recipient).unwrap();
    let model = h.app.world().get::<ModelDisplay>(recipient).copied();
    let position = *h.app.world().get::<Position>(recipient).unwrap();
    assert_eq!(activate(&mut h, SPOTLIGHT)[0].error, None);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(recipient)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SPOTLIGHT_SPELL)
        .unwrap();
    assert_eq!(aura.caster, owner);
    assert_eq!(aura.duration_ms, 1_800_000);
    assert_eq!(aura.effects[0].amount, 0.0);
    assert!(
        h.app
            .world()
            .get::<UnitAuras>(recipient)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == SPOTLIGHT_SPELL && aura.caster == Some(owner.to_bits()))
    );
    for unit in [owner, foreign] {
        assert!(!crate::auras::has_aura(
            h.app.world(),
            unit,
            SPOTLIGHT_SPELL
        ));
    }
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SPOTLIGHT_SPELL));
    assert!(
        h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == SPOTLIGHT_SPELL && cd.duration_ms == 3_600_000 && !cd.is_gcd)
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(recipient_connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura {
            spell_id: SPOTLIGHT_SPELL,
        });
    h.advance(5);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        recipient,
        SPOTLIGHT_SPELL
    ));
    assert!(
        !h.app
            .world()
            .get::<UnitAuras>(recipient)
            .unwrap()
            .auras
            .iter()
            .any(|aura| aura.spell_id == SPOTLIGHT_SPELL)
    );
    assert_eq!(h.app.world().get::<Health>(recipient), Some(&health));
    assert_eq!(h.app.world().get::<ModelDisplay>(recipient).copied(), model);
    assert_eq!(h.app.world().get::<Position>(recipient), Some(&position));
    assert_eq!(
        activate(&mut h, SPOTLIGHT)[0].error.as_deref(),
        Some("OnCooldown")
    );
}

#[test]
fn toy_coverage_spotlight_foreign_ally_refuses_without_caster_substitution_or_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("spotlight-foreign.redb"));
    install_visual_source(&mut h);
    let foreign = spawn_spotlight_friend(&mut h, character + 1, 2.0);
    learn_effect_toy(&mut h, character, SPOTLIGHT);
    h.select(Some(foreign));
    h.advance(3);
    assert_eq!(
        activate(&mut h, SPOTLIGHT)[0].error.as_deref(),
        Some("InvalidTarget")
    );
    for unit in [h.caster, foreign] {
        assert!(!crate::auras::has_aura(
            h.app.world(),
            unit,
            SPOTLIGHT_SPELL
        ));
    }
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == SPOTLIGHT_SPELL));
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|cd| cd.spell_id == SPOTLIGHT_SPELL)
    );
}

#[test]
fn toy_coverage_spotlight_group_eligible_visual_or_payload_drift_refuses() {
    for absent_visual in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (mut h, character) = setup(&dir.path().join("spotlight-drift.redb"));
        install_visual_source(&mut h);
        let recipient = spawn_spotlight_friend(&mut h, character + 1, 2.0);
        invite_spotlight_friend(&mut h, character, recipient);
        learn_effect_toy(&mut h, character, SPOTLIGHT);
        h.select(Some(recipient));
        h.advance(3);
        let mut changed = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(SPOTLIGHT_SPELL)
            .unwrap()
            .clone();
        if absent_visual {
            changed.has_spell_visual = false;
        } else {
            changed.effects[0].base_points = 1.0;
        }
        h.app
            .insert_resource(crate::spell_info::SpellStore::fixture(vec![changed]));
        assert_eq!(
            activate(&mut h, SPOTLIGHT)[0].error.as_deref(),
            Some("unsupported toy spell: aura 4")
        );
        for unit in [h.caster, recipient] {
            assert!(!crate::auras::has_aura(
                h.app.world(),
                unit,
                SPOTLIGHT_SPELL
            ));
        }
        assert!(!h.inbox.go.iter().any(|go| go.spell_id == SPOTLIGHT_SPELL));
        assert!(
            !h.inbox
                .cooldowns
                .iter()
                .any(|cd| cd.spell_id == SPOTLIGHT_SPELL)
        );
    }
}

#[test]
fn toy_coverage_spotlight_census_retains_heating_sound_audience_and_dummy_gaps() {
    let report = report_toy_census(&crate::test_data::world_db_path(), "TOY_SPOTLIGHT_REPORT");
    let rows = report["toys"].as_array().unwrap();
    for item in [SPOTLIGHT, 279052, 136928] {
        let row = rows.iter().find(|row| row["item_id"] == item).unwrap();
        assert!(row["unavailable"].is_null(), "{row}");
    }
    for item in [180873, 205936, 86565, 18986] {
        let row = rows.iter().find(|row| row["item_id"] == item).unwrap();
        assert!(!row["unavailable"].is_null(), "{row}");
    }
}
