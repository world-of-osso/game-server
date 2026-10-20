//! TC a352b1fa `SpellHistory::SaveToDB`/`LoadFromDB` keep a player's spell and category
//! cooldowns as wallclock deadlines across logout and server restart, and
//! `SendSpellHistory` replays the active ones at login. Prismatic Sphere (28027,
//! Outland Enchanting trainer 828, rank 25) has the 48 h Enchanting category 1174;
//! Berserker Rage (18499) has a 60 s spell cooldown and no category.

use std::path::Path;

use super::*;

const PRISMATIC_SPHERE: u32 = 28027;
const PRISMATIC_SPHERE_ITEM: u32 = 22460;
const LARGE_PRISMATIC_SHARD: u32 = 22449;
/// Totem category 41, Prismatic Sphere's required tool (`spell_totems`).
const RUNED_FEL_IRON_ROD: u32 = 22461;
const ENCHANTING_CATEGORY: u32 = 1174;
const SPHERE_COOLDOWN_MS: u32 = 172_800_000;
/// Johan Barnes, Honor Hold (Alliance); Felannia 18753 is Horde.
const ENCHANTING_TRAINER_NPC: u32 = 18773;
const ENCHANTING: u32 = 264457;
const OUTLAND_ENCHANTING: u32 = 264461;
const OUTLAND_ENCHANTING_SKILL: u32 = 2493;
const BERSERKER_RAGE: u32 = 18499;
const BERSERKER_RAGE_COOLDOWN_MS: u32 = 60_000;

/// A harness whose character database is a file, so it can be reopened.
fn file_harness(path: &Path) -> Harness {
    let db = redb::Database::create(path).unwrap();
    persistence::create_tables(&db);
    let mut h = harness_with(10, |app| {
        app.insert_resource(GameDatabase::new(db));
    })
    .unwrap();
    install_spells(&mut h.app, &h.conn);
    h
}

/// The default profession spells plus the Enchanting trainer's and Berserker Rage.
fn install_spells(app: &mut App, conn: &Connection) {
    let data = world_db().unwrap();
    let mut ids = profession_spells(conn);
    let mut statement = conn
        .prepare(
            "SELECT SpellId FROM tdb_trainer_spell WHERE TrainerId = 828 \
             UNION SELECT EffectTriggerSpell FROM spell_effect WHERE Effect = 36 AND SpellID IN \
               (SELECT SpellId FROM tdb_trainer_spell WHERE TrainerId = 828) \
             UNION SELECT Spell FROM skill_line_ability WHERE SkillLine = 333",
        )
        .unwrap();
    ids.extend(
        statement
            .query_map([], |row| row.get::<_, u32>(0))
            .unwrap()
            .map(Result::unwrap),
    );
    ids.push(BERSERKER_RAGE);
    let spells = ids
        .into_iter()
        .filter_map(|id| data.store.get(id).cloned())
        .collect();
    app.insert_resource(SpellStore::fixture(spells));
}

/// Leave the world through the real disconnect path (`queue_player_despawn`).
fn leave_world(h: &mut Harness) {
    let mut commands = h.app.world_mut().commands();
    crate::networking::queue_player_despawn(&mut commands, h.player);
    h.app.world_mut().flush();
    assert!(h.app.world_mut().despawn(h.connection));
    assert!(h.app.world_mut().despawn(h.player));
}

fn relog(h: &mut Harness) {
    h.take::<SpellCooldownUpdate>();
    leave_world(h);
    (h.player, h.connection) = enter_world(&mut h.app, h.character_id, h.level);
}

/// Log out, close the character database, and enter again on a new server app.
fn restart(mut h: Harness, path: &Path) -> Harness {
    leave_world(&mut h);
    let (character_id, level) = (h.character_id, h.level);
    drop(
        h.app
            .world_mut()
            .remove_resource::<GameDatabase>()
            .unwrap()
            .into_database(),
    );
    drop(h);
    let reopened = redb::Database::open(path).unwrap();
    let (mut app, conn) = harness_app(|app| {
        app.insert_resource(GameDatabase::new(reopened));
    })
    .unwrap();
    install_spells(&mut app, &conn);
    let (player, connection) = enter_world(&mut app, character_id, level);
    Harness {
        app,
        conn,
        character_id,
        player,
        connection,
        level,
    }
}

/// The single cooldown the server replays at world entry, still counting down.
fn assert_replayed(h: &mut Harness, spell_id: u32, category: u32, full_ms: u32, when: &str) {
    h.advance(3);
    let replayed = h.take::<SpellCooldownUpdate>();
    let [update] = replayed.as_slice() else {
        panic!("{when}: expected one replayed cooldown, got {replayed:?}")
    };
    assert_eq!(
        (update.spell_id, update.category, update.is_gcd),
        (spell_id, category, false),
        "{when}"
    );
    assert!(
        update.remaining_ms > full_ms - 60_000 && update.remaining_ms <= full_ms,
        "{when}: {update:?}"
    );
}

/// Johan Barnes (npcflag 209) is also a vendor, so pick the trainer option
/// (TC `GOSSIP_ICON_TRAINER` 3) from his role-built gossip menu.
fn open_enchanting_trainer(h: &mut Harness) -> Entity {
    let npc = h.spawn_npc(ENCHANTING_TRAINER_NPC);
    h.send::<_, InteractionChannel>(InteractNpc { npc: npc.to_bits() });
    let opened = h.take::<InteractionOpened>();
    let [
        InteractionOpened {
            kind: InteractionKind::Gossip(menu),
            ..
        },
    ] = opened.as_slice()
    else {
        panic!("{opened:?}")
    };
    let option = menu
        .options
        .iter()
        .find(|option| option.icon == 3)
        .unwrap_or_else(|| panic!("{menu:?}"));
    h.send::<_, InteractionChannel>(SelectGossipOption {
        npc: npc.to_bits(),
        option_id: option.option_id,
    });
    h.advance(3);
    npc
}

fn learn_prismatic_sphere(h: &mut Harness) {
    h.set_gold(1000);
    let npc = open_enchanting_trainer(h);
    for spell in [ENCHANTING, OUTLAND_ENCHANTING] {
        h.buy(npc, spell);
        let failed = h.take::<TrainerBuyFailed>();
        assert!(failed.is_empty(), "{spell}: {failed:?}");
    }
    h.set_rank(OUTLAND_ENCHANTING_SKILL, 25);
    h.buy(npc, PRISMATIC_SPHERE);
    assert!(h.take::<TrainerBuyFailed>().is_empty());
    assert!(h.professions().knows(PRISMATIC_SPHERE));
}

fn assert_sphere_rejected(h: &mut Harness, when: &str) {
    h.take::<CastFailed>();
    h.craft(PRISMATIC_SPHERE, 1);
    h.finish_crafting();
    let failed: Vec<_> = h
        .take::<CastFailed>()
        .into_iter()
        .map(|failed| (failed.spell_id, failed.reason))
        .collect();
    assert_eq!(
        failed,
        vec![(PRISMATIC_SPHERE, CastFailReason::OnCooldown)],
        "{when}"
    );
    assert_eq!(h.bag_count(LARGE_PRISMATIC_SHARD), 8, "{when}: shards kept");
    assert_eq!(
        h.bag_count(PRISMATIC_SPHERE_ITEM),
        1,
        "{when}: no second sphere"
    );
}

#[test]
fn spell_cooldown_recipe_rejects_repeat_in_session_after_relog_and_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recipe-cooldown.redb");
    let mut h = file_harness(&path);
    learn_prismatic_sphere(&mut h);
    h.give(RUNED_FEL_IRON_ROD, "Runed Fel Iron Rod", 1);
    h.give(LARGE_PRISMATIC_SHARD, "Large Prismatic Shard", 12);
    h.take::<CastFailed>();
    h.take::<SpellCooldownUpdate>();
    h.craft(PRISMATIC_SPHERE, 1);
    h.finish_crafting();
    let failed = h.take::<CastFailed>();
    assert!(failed.is_empty(), "{failed:?}");
    assert_eq!(h.bag_count(PRISMATIC_SPHERE_ITEM), 1);
    assert_eq!(h.bag_count(LARGE_PRISMATIC_SHARD), 8);
    let started = h.take::<SpellCooldownUpdate>();
    assert!(
        started.contains(&SpellCooldownUpdate {
            spell_id: PRISMATIC_SPHERE,
            category: ENCHANTING_CATEGORY,
            duration_ms: SPHERE_COOLDOWN_MS,
            remaining_ms: SPHERE_COOLDOWN_MS,
            is_gcd: false,
        }),
        "{started:?}"
    );

    assert_sphere_rejected(&mut h, "in session");
    relog(&mut h);
    assert_replayed(
        &mut h,
        PRISMATIC_SPHERE,
        ENCHANTING_CATEGORY,
        SPHERE_COOLDOWN_MS,
        "relog",
    );
    assert_sphere_rejected(&mut h, "after relog");
    let mut h = restart(h, &path);
    assert_replayed(
        &mut h,
        PRISMATIC_SPHERE,
        ENCHANTING_CATEGORY,
        SPHERE_COOLDOWN_MS,
        "restart",
    );
    assert_sphere_rejected(&mut h, "after restart");
}

fn know_berserker_rage(h: &mut Harness) {
    h.app
        .world_mut()
        .entity_mut(h.player)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            BERSERKER_RAGE,
        ]));
    h.advance(3);
}

fn cast_berserker_rage(h: &mut Harness) -> Vec<(u32, CastFailReason)> {
    h.take::<CastFailed>();
    h.send::<SpellCastIntent, CombatChannel>(SpellCastIntent {
        destination: None,
        spell_id: Some(BERSERKER_RAGE),
        spell: String::new(),
        target_entity: None,
        target_item_guid: None,
        witness: None,
    });
    // Past the 1.5 s global cooldown, so only the spell's own cooldown can reject.
    h.advance(40);
    h.take::<CastFailed>()
        .into_iter()
        .map(|failed| (failed.spell_id, failed.reason))
        .collect()
}

#[test]
fn spell_cooldown_class_spell_rejects_repeat_in_session_after_relog_and_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("class-cooldown.redb");
    let mut h = file_harness(&path);
    know_berserker_rage(&mut h);
    h.take::<SpellCooldownUpdate>();
    assert_eq!(cast_berserker_rage(&mut h), vec![]);
    let started = h.take::<SpellCooldownUpdate>();
    assert!(
        started.contains(&SpellCooldownUpdate {
            spell_id: BERSERKER_RAGE,
            category: 0,
            duration_ms: BERSERKER_RAGE_COOLDOWN_MS,
            remaining_ms: BERSERKER_RAGE_COOLDOWN_MS,
            is_gcd: false,
        }),
        "{started:?}"
    );
    let rejected = vec![(BERSERKER_RAGE, CastFailReason::OnCooldown)];
    assert_eq!(cast_berserker_rage(&mut h), rejected, "in session");

    relog(&mut h);
    know_berserker_rage(&mut h);
    assert_replayed(
        &mut h,
        BERSERKER_RAGE,
        0,
        BERSERKER_RAGE_COOLDOWN_MS,
        "relog",
    );
    assert_eq!(cast_berserker_rage(&mut h), rejected, "after relog");
    let mut h = restart(h, &path);
    know_berserker_rage(&mut h);
    assert_replayed(
        &mut h,
        BERSERKER_RAGE,
        0,
        BERSERKER_RAGE_COOLDOWN_MS,
        "restart",
    );
    assert_eq!(cast_berserker_rage(&mut h), rejected, "after restart");
}
