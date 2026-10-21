//! Current Retail LittleWigs2777a9d Kokia.lua + actual70448 native targets/effects.
use super::*;

const KOKIA: u32 = 189232;
const MAP: u32 = 2521;
const BIT: u32 = 0;
const SEARING_BLOWS: u32 = 372858;

fn kokia_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(KOKIA, MAP, BIT, difficulty, None);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

#[test]
fn encounter_coverage_kokia_current_retail_inputs_qualify() {
    let conn =
        Connection::open_with_flags(world_db_path(), OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    for (spell, expected) in [
        (372864, (3, 0, 0)),
        (372110, (3, 0, 372107)),
        (SEARING_BLOWS, (6, 23, 372859)),
    ] {
        let actual:(u32,u32,u32)=conn.query_row("SELECT Effect,EffectAura,EffectTriggerSpell FROM spell_effect WHERE SpellID=? AND DifficultyID=0 AND EffectIndex=0",[spell],|row|Ok((row.get(0)?,row.get(1)?,row.get(2)?))).unwrap();
        assert_eq!(actual, expected, "current native effect/child{spell}");
    }
    assert_eq!(
        conn.query_row(
            "SELECT MapID,Bit FROM dungeon_encounter WHERE ID=2606",
            [],
            |row| Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
        )
        .unwrap(),
        (MAP, BIT)
    );
}

#[test]
fn encounter_coverage_kokia_native_searing_periodic_weapon_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = kokia_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(
            h.app.world_mut(),
            boss,
            SEARING_BLOWS,
            Some(player),
            false,
        )
        .expect("ordinary native372858 admission");
        h.advance(3.5);
        assert_eq!(cast_count(&h, boss, SEARING_BLOWS), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.spell_id == Some(372859)),
            "real native aura23→372859 weapon damage in difficulty{difficulty}"
        );
        assert!(!h.has_aura(h.players[1].0, SEARING_BLOWS));
    }
}

#[test]
#[ignore = "KOKIA_CURRENT_RETAIL_PORT_PENDING"]
fn encounter_coverage_kokia_first_searing_blows_follows_current_retail_clock() {
    let (mut h, boss, _) = kokia_copy(205);
    h.engage(boss);
    h.advance(32.0);
    assert_eq!(
        cast_count(&h, boss, SEARING_BLOWS),
        1,
        "current Retail28s ability request, not legacy21.6s"
    );
}
