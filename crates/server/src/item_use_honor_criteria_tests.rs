//! Installed70391 CriteriaTree6273 requires ten real HKs with aura48890.
use super::*;
use crate::achievement::{AchievementPlugin, PlayerAchievementState};
use crate::persistence::{ClientId, GameDatabase};
use crate::spell_cast::effects::{EffectCtx, deal_damage};
use shared::components::{CurrentPvpFaction, Level, UnitFactionTemplate};

const CANDY: u32 = 37583;
const BUFF: u32 = 48890;
const CRITERION: u32 = 3882;
const ACHIEVEMENT: u32 = 1261;
const REQUIRED_KILLS: u64 = 10;

#[test]
fn achievements_criteria_gnerd_native_item_ten_zero_xp_honor_kills_and_reopen() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut eater = new_gnerd_eater(file.path());
    let player = eater.h.caster;
    use_slot(&mut eater, 0);
    assert!(errors(&mut eater).is_empty());
    assert_eq!(count_of(&eater, CANDY), 0);
    // Authored CastingTimeIndex4 is1000ms; accepting UseItem precedes its aura.
    eater.h.advance(20);
    assert!(
        eater
            .h
            .app
            .world()
            .get::<ActiveAuras>(player)
            .unwrap()
            .has_spell(BUFF)
    );
    assert_eq!(gnerd_counter(&eater), None);
    eater.h.advance(5);
    assert_eq!(gnerd_counter(&eater), None, "idle is not a kill");
    for count in 1..=REQUIRED_KILLS {
        let victim = spawn_honor_victim(&mut eater);
        damage_honor_victim(&mut eater, victim, 1.0);
        assert_eq!(gnerd_counter(&eater), (count > 1).then_some(count - 1));
        damage_honor_victim(&mut eater, victim, 2000.0);
        assert_eq!(eater.h.health(victim), 0.0);
        assert_eq!(gnerd_counter(&eater), Some(count));
        let state = eater
            .h
            .app
            .world()
            .get::<PlayerAchievementState>(player)
            .unwrap();
        assert_eq!(
            state.earned_ids.contains(&ACHIEVEMENT),
            count == REQUIRED_KILLS
        );
        damage_honor_victim(&mut eater, victim, 2000.0);
        assert_eq!(gnerd_counter(&eater), Some(count), "dead-target replay");
    }
    let progress = &eater
        .h
        .app
        .world()
        .get::<crate::pvp::PlayerPvpState>(player)
        .unwrap()
        .progress;
    assert_eq!(
        progress.lifetime_honor_xp, 0,
        "AddHonorXP rejects level1, not the HK"
    );
    assert_eq!(progress.honor, 0);
    assert_eq!(
        eater
            .h
            .app
            .world()
            .get::<crate::currency::PlayerCurrencies>(player)
            .unwrap()
            .0
            .get(1792),
        0,
        "zero XP eligible kills credit type35 but mirror zero wallet Honor"
    );
    assert!(progress.honor_level.is_none());
    let date = eater
        .h
        .app
        .world()
        .get::<PlayerAchievementState>(player)
        .unwrap()
        .earned_at[&ACHIEVEMENT];
    eater
        .h
        .app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::CancelAura>>(eater.h.connection)
        .unwrap()
        .send::<shared::protocol::CombatChannel>(shared::protocol::CancelAura { spell_id: BUFF });
    eater.h.advance(5);
    assert!(
        !eater
            .h
            .app
            .world()
            .get::<ActiveAuras>(player)
            .unwrap()
            .has_spell(BUFF)
    );
    let victim = spawn_honor_victim(&mut eater);
    damage_honor_victim(&mut eater, victim, 2000.0);
    assert_eq!(
        gnerd_counter(&eater),
        Some(REQUIRED_KILLS),
        "every kill still requires the buff"
    );
    let character = eater.character_id;
    drop(eater);
    let db = redb::Database::open(file.path()).unwrap();
    let saved = persistence::load_achievements(&db, character);
    assert_eq!(
        saved.criteria_progress.get(&CRITERION),
        Some(&REQUIRED_KILLS)
    );
    assert!(saved.earned_ids.contains(&ACHIEVEMENT));
    assert_eq!(saved.earned_at.get(&ACHIEVEMENT), Some(&date));
}

fn new_gnerd_eater(path: &std::path::Path) -> Eater {
    let mut eater = eater_with_spells(&[CANDY], 1, &[BUFF, 133], MAGE)
        .expect("sourced G.N.E.R.D.S. item/spell");
    let inventory = persistence::load_inventory(
        &eater.h.app.world().resource::<GameDatabase>().db,
        eater.character_id,
    );
    let db = redb::Database::create(path).unwrap();
    persistence::create_tables(&db);
    let (character, _) = persistence::create_character(&db, 1, "Eater", 1, MAGE).unwrap();
    assert_eq!(character, eater.character_id);
    persistence::save_inventory(&db, character, &inventory);
    eater.h.app.insert_resource(GameDatabase { db });
    let conn = rusqlite::Connection::open(world_db()).unwrap();
    eater
        .h
        .app
        .insert_resource(crate::achievement::load_achievement_catalog(&conn).unwrap());
    bevy::app::Plugin::build(&AchievementPlugin, &mut eater.h.app);
    eater.h.app.world_mut().entity_mut(eater.h.caster).insert((
        ClientId(character),
        PlayerAchievementState::default(),
        crate::pvp::PlayerPvpState::default(),
        CurrentPvpFaction(2),
        Level(1),
    ));
    eater.h.advance(1);
    eater
}

fn spawn_honor_victim(eater: &mut Eater) -> Entity {
    let victim = spawn_player(&mut eater.h.app, 1, 3.0);
    eater
        .h
        .app
        .world_mut()
        .get_mut::<shared::components::Player>(victim)
        .unwrap()
        .race = 2;
    eater.h.app.world_mut().entity_mut(victim).insert((
        CurrentPvpFaction(1),
        UnitFactionTemplate(2),
        Level(1),
    ));
    victim
}

fn damage_honor_victim(eater: &mut Eater, victim: Entity, amount: f32) {
    let data = crate::spell_cast::test_harness::world_db().unwrap();
    let ctx = EffectCtx {
        caster: eater.h.caster,
        spell: data.store.get(133).unwrap(),
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    deal_damage(eater.h.app.world_mut(), &ctx, victim, amount, false, false);
}

fn gnerd_counter(eater: &Eater) -> Option<u64> {
    eater
        .h
        .app
        .world()
        .get::<PlayerAchievementState>(eater.h.caster)
        .unwrap()
        .criteria_progress
        .get(&CRITERION)
        .copied()
}
