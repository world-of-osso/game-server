use super::*;
use crate::spell_cast::test_harness::{Harness, drain, harness};
use lightyear::prelude::{MessageReceiver, MessageSender};
use rusqlite::OpenFlags;
use shared::protocol::{ChatMessage, ChatType};

const CAGE: InMap = InMap {
    map_id: MAP,
    instance_id: 0,
};
/// TDB areatrigger spawns 269/270 ("Entrance Purple"/"Entrance Gold").
const PURPLE_ENTRANCE: Vec3 = Vec3::new(443.727, 314.306, -24.5936);
const GOLD_ENTRANCE: Vec3 = Vec3::new(443.885, 456.386, -24.5629);

fn cage_harness(team: BgTeam) -> Harness {
    let mut h = harness(
        Vec::new(),
        &[STARTING_AREA_MARKER, WARNING, WARNING_TELEPORT],
        1,
    )
    .unwrap();
    let db = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app.insert_resource(Catalog::load(&db).unwrap());
    let battleground = shared::protocol::BattlegroundSnapshot {
        match_id: 1,
        battleground_id: 1117,
        name: "Cage of Carnage".into(),
        map_id: MAP,
        team,
        phase: BgPhase::InProgress,
        accepted: true,
        invitation_time_left_secs: None,
        score_a: 0,
        score_b: 0,
        flags: Default::default(),
        world_states: Vec::new(),
        outcome: None,
        players: Vec::new(),
        honor_reward: None,
        arena: None,
    };
    h.app.world_mut().entity_mut(h.caster).insert((
        CAGE,
        super::super::PlayerPvpState {
            battleground: Some(battleground),
            ..Default::default()
        },
    ));
    h
}

fn place(h: &mut Harness, point: Vec3) {
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::game_objects::bevy_position(point));
}

fn location(h: &Harness) -> Vec3 {
    crate::game_objects::wow_position(h.app.world().get::<Position>(h.caster).unwrap())
}

fn has(h: &Harness, spell_id: u32) -> bool {
    crate::auras::has_aura(h.app.world(), h.caster, spell_id)
}

#[test]
fn arena_cage_entrance_marker_warns_after_start_then_teleports_alliance_to_purple_exit() {
    let mut h = cage_harness(BgTeam::A);
    let mut clock = Clock::default();
    place(&mut h, PURPLE_ENTRANCE);
    update_presence(h.app.world_mut(), CAGE, &mut clock);
    assert!(has(&h, STARTING_AREA_MARKER));
    update_warnings(h.app.world_mut(), CAGE, 4.999, &mut clock);
    assert!(
        !has(&h, WARNING),
        "first source check is 5s after the start"
    );
    update_warnings(h.app.world_mut(), CAGE, 5.0, &mut clock);
    assert!(has(&h, WARNING));
    // Warning lasts DurationIndex28 = 5000ms; its removal while marked teleports.
    h.advance(110);
    assert!(!has(&h, WARNING));
    let at = location(&h);
    assert!(
        at.distance(PURPLE_WARNING_TELEPORT.0) < 0.01,
        "Alliance camper must land at the Purple exit, got {at:?}"
    );
}

#[test]
fn arena_cage_leaving_gold_entrance_drops_marker_so_warning_expiry_does_not_teleport() {
    let mut h = cage_harness(BgTeam::B);
    let mut clock = Clock::default();
    place(&mut h, GOLD_ENTRANCE);
    update_presence(h.app.world_mut(), CAGE, &mut clock);
    update_warnings(h.app.world_mut(), CAGE, 5.0, &mut clock);
    assert!(has(&h, STARTING_AREA_MARKER) && has(&h, WARNING));
    // Gold box: facing 4.743, half-length 35, so 56yd along -Y is outside it.
    let outside = GOLD_ENTRANCE - Vec3::Y * 56.0;
    place(&mut h, outside);
    update_presence(h.app.world_mut(), CAGE, &mut clock);
    assert!(!has(&h, STARTING_AREA_MARKER));
    h.advance(110);
    assert!(!has(&h, WARNING));
    assert!(location(&h).distance(outside) < 0.01);
    update_warnings(h.app.world_mut(), CAGE, 10.0, &mut clock);
    assert!(!has(&h, WARNING), "unmarked players are not warned again");
}

#[test]
fn arena_cage_gold_camper_teleports_to_gold_exit() {
    let mut h = cage_harness(BgTeam::B);
    let mut clock = Clock::default();
    place(&mut h, GOLD_ENTRANCE + Vec3::new(3.0, 20.0, 0.0));
    update_presence(h.app.world_mut(), CAGE, &mut clock);
    update_warnings(h.app.world_mut(), CAGE, 5.0, &mut clock);
    h.advance(110);
    assert!(location(&h).distance(GOLD_WARNING_TELEPORT.0) < 0.01);
}

const ANNOUNCER_SPELLS: [u32; 7] = [
    REACTION_TRIGGER,
    REACTION_LOW_HEALTH,
    VO_COOLDOWN,
    VO_COOLDOWN_DEATH,
    LOW_HEALTH_COOLDOWN,
    2948,
    STARTING_AREA_MARKER,
];

/// Source spawn 232750 with its TDB creature_text groups, plus a chat-capable link.
fn announcer_harness() -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &ANNOUNCER_SPELLS, 1).unwrap();
    let db = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::scripted_ai::creature_text::load_creature_texts(&db).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert(CAGE);
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<ChatMessage>::default(),
        MessageReceiver::<ChatMessage>::default(),
        MessageSender::<shared::protocol::CreatureTextSound>::default(),
        MessageReceiver::<shared::protocol::CreatureTextSound>::default(),
    ));
    let naminze = h
        .app
        .world_mut()
        .spawn((
            shared::components::Npc {
                template_id: NAMINZE,
                name: "Naminze Boltfingers".into(),
            },
            // TDB creature_template_model: display123368 is Gender1 (female).
            shared::components::ModelDisplay {
                display_id: 123_368,
            },
            CAGE,
            crate::game_objects::bevy_position(Vec3::new(492.082, 384.34, -14.6007)),
            Health {
                current: 100.0,
                max: 100.0,
            },
        ))
        .id();
    (h, naminze)
}

/// Chat already queued plus whatever the next frames deliver; a receiver only
/// holds a frame's messages, so drain after every frame.
fn said(h: &mut Harness) -> Vec<String> {
    let connection = h.connection;
    let mut lines = Vec::new();
    for frame in 0..3 {
        if frame > 0 {
            h.advance(1);
        }
        lines.extend(
            drain::<ChatMessage>(&mut h.app, connection)
                .into_iter()
                .map(|message| {
                    assert!(matches!(message.channel, ChatType::MonsterSay(_)));
                    assert_eq!(message.sender, "Naminze Boltfingers");
                    message.content
                }),
        );
    }
    lines
}

/// Advance `frames`, collecting chat delivered on each.
fn advance_saying(h: &mut Harness, frames: usize) -> Vec<String> {
    let mut lines = Vec::new();
    for _ in 0..frames {
        h.advance(1);
        lines.extend(said(h));
    }
    lines
}

const ALMOST_DEAD: [&str; 4] = [
    "Ah, c'mon. Right when it was gettin' good!",
    "This is a gen-u-ine nailbiter!",
    "Aw, you keep spoilin' all my fun!",
    "Keepin' this fight interestin' this time!",
];
const KILL: [&str; 3] = [
    "That's gotta hurt. But not for long. Heh.",
    "Wowza. I'm gonna remember that one for a long time.",
    "Oof, right in the... well, the everywhere!",
];

fn set_health(h: &mut Harness, current: f32) {
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = current;
}

#[test]
fn arena_cage_announcer_reacts_once_to_entrant_falling_below_twenty_percent() {
    let (mut h, naminze) = announcer_harness();
    update_joined(h.app.world_mut(), CAGE, &mut Clock::default());
    assert!(has(&h, REACTION_TRIGGER));
    h.advance(2);
    assert!(
        said(&mut h).is_empty(),
        "full health entrant triggers nothing"
    );
    set_health(&mut h, 210.0);
    assert!(advance_saying(&mut h, 2).is_empty());
    assert!(!has(&h, LOW_HEALTH_COOLDOWN));
    set_health(&mut h, 190.0);
    let lines = advance_saying(&mut h, 2);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(ALMOST_DEAD.contains(&lines[0].as_str()), "{lines:?}");
    assert!(has(&h, LOW_HEALTH_COOLDOWN));
    assert!(crate::auras::has_aura(h.app.world(), naminze, VO_COOLDOWN));
    // A second crossing inside both 10s cooldown auras stays silent.
    set_health(&mut h, 900.0);
    assert!(advance_saying(&mut h, 2).is_empty());
    set_health(&mut h, 100.0);
    assert!(advance_saying(&mut h, 2).is_empty());
}

#[test]
fn arena_cage_announcer_reacts_to_entrant_death_through_source_proc() {
    let (mut h, naminze) = announcer_harness();
    update_joined(h.app.world_mut(), CAGE, &mut Clock::default());
    h.advance(1);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(2948)
        .unwrap()
        .clone();
    let ctx = crate::spell_cast::effects::EffectCtx {
        caster: h.npc,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let victim = h.caster;
    crate::spell_cast::effects::deal_damage(h.app.world_mut(), &ctx, victim, 5000.0, false, false);
    let lines = advance_saying(&mut h, 3);
    assert!(
        lines.iter().any(|line| KILL.contains(&line.as_str())),
        "{lines:?}"
    );
    assert!(crate::auras::has_aura(
        h.app.world(),
        naminze,
        VO_COOLDOWN_DEATH
    ));
}

#[test]
fn arena_cage_announcer_speaks_at_preparation_stage_two_and_start() {
    let (mut h, _) = announcer_harness();
    let mut clock = Clock::default();
    let preparing = MatchTimes {
        preparation_started: Some(100.0),
        started: None,
    };
    update_announcements(h.app.world_mut(), CAGE, 129.9, preparing, &mut clock);
    assert!(said(&mut h).is_empty());
    update_announcements(h.app.world_mut(), CAGE, 130.0, preparing, &mut clock);
    update_announcements(h.app.world_mut(), CAGE, 140.0, preparing, &mut clock);
    assert_eq!(
        said(&mut h),
        ["He he he he. Look how tough all of you look. This is gonna be good!"]
    );
    let running = MatchTimes {
        preparation_started: Some(100.0),
        started: Some(160.0),
    };
    update_announcements(h.app.world_mut(), CAGE, 160.0, running, &mut clock);
    update_announcements(h.app.world_mut(), CAGE, 170.0, running, &mut clock);
    assert_eq!(said(&mut h), ["Get ready. It's about to go down!"]);
}

#[test]
fn arena_cage_announcer_delivers_source_sound_kit_with_its_preparation_line() {
    let (mut h, naminze) = announcer_harness();
    crate::scripted_ai::creature_text::talk(h.app.world_mut(), naminze, TEXT_PREPARE);
    let mut sounds = Vec::new();
    for _ in 0..3 {
        h.advance(1);
        let connection = h.connection;
        sounds.extend(drain::<shared::protocol::CreatureTextSound>(
            &mut h.app, connection,
        ));
    }
    assert_eq!(
        sounds,
        [shared::protocol::CreatureTextSound {
            speaker: naminze.to_bits(),
            sound_kit: 269092,
        }]
    );
}
