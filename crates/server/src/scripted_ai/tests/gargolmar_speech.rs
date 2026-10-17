//! TC a352b1fa Gargolmar OnSpellCast30642 Talk1 and KilledUnit Talk4.
use super::*;

fn authored_yells(h: &Harness, boss: Entity, group: u8) -> usize {
    let texts = &h.app.world().resource::<CreatureTexts>().groups[&17306][&group];
    let name = &h
        .app
        .world()
        .get::<shared::components::Npc>(boss)
        .unwrap()
        .name;
    h.inbox(0)
        .chat
        .iter()
        .filter(|line| {
            let authored = texts
                .iter()
                .any(|text| line.content == text.male_text || line.content == text.female_text);
            let from_boss = line.channel == shared::protocol::ChatType::MonsterYell(boss.to_bits())
                && &line.sender == name;
            authored && from_boss
        })
        .count()
}

#[test]
#[ignore = "source successful native30642 Talk1 acceptance pending implementation"]
fn encounter_coverage_gargolmar_speech_heal_request_success_speaks_once() {
    let (mut h, boss, _) = super::gargolmar::pull_gargolmar();
    assert_eq!(authored_yells(&h, boss, 1), 0);
    h.hit_pct(boss, 61.0);
    h.advance(0.2);
    assert_eq!(
        spell_gos(&h, boss, 30642).len(),
        1,
        "actual incoming-below40 source task completes ordinary30642"
    );
    assert_eq!(
        authored_yells(&h, boss, 1),
        1,
        "successful source30642 must emit authored heal-request Talk1"
    );
    h.hit_pct(boss, 1.0);
    h.advance(1.0);
    assert_eq!(
        spell_gos(&h, boss, 30642).len(),
        1,
        "subsequent damage does not rearm source one-shot"
    );
    assert_eq!(
        authored_yells(&h, boss, 1),
        1,
        "later damage and aura ticks cannot repeat cast speech"
    );
    assert!(
        h.inbox(1).chat.is_empty(),
        "other copy cannot hear source speech"
    );
}

#[test]
fn encounter_coverage_gargolmar_speech_death_preempts_pending_heal_request() {
    let (mut h, boss, _) = super::gargolmar::pull_gargolmar();
    h.hit_pct(boss, 61.0);
    h.hit_pct(boss, 200.0);
    h.advance(0.2);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(boss)
            .is_some()
    );
    assert!(
        spell_gos(&h, boss, 30642).is_empty(),
        "source task never completes after actual lethal damage"
    );
    assert_eq!(
        authored_yells(&h, boss, 1),
        0,
        "scheduling or taking damage is not successful OnSpellCast"
    );
    assert!(h.inbox(1).chat.is_empty());
}

#[test]
#[ignore = "source native melee KilledUnit Talk4 acceptance pending implementation"]
fn encounter_coverage_gargolmar_speech_actual_melee_kill_speaks_once() {
    let (mut h, boss, _) = super::gargolmar::pull_gargolmar();
    super::omor_slay::install_player_death_transition(&mut h);
    let player = h.player();
    h.app.world_mut().get_mut::<Health>(player).unwrap().current = 1.0;
    h.advance(2.5); // Before earliest source Surge3s/MortalWound5s; native melee only.
    assert_eq!(
        h.health(player).current,
        0.0,
        "actual sourced melee kills wounded victim"
    );
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.spell_id.is_none()
                && event.amount > 0
        }),
        "real ordinary melee damage, not a substitute spell or direct kill"
    );
    let corpse = h
        .app
        .world()
        .get::<crate::death::PlayerDeathState>(player)
        .expect("actual player death transition");
    assert_eq!(corpse.state, shared::death::DeathState::Dead);
    assert!(corpse.corpse.is_some());
    assert_eq!(
        authored_yells(&h, boss, 4),
        1,
        "actual killer Gargolmar must emit authored slay Talk4"
    );
    h.advance(1.0);
    assert_eq!(
        authored_yells(&h, boss, 4),
        1,
        "dead victim cannot produce repeated slay"
    );
    assert!(h.inbox(1).chat.is_empty());
}

#[test]
#[ignore = "source ordinary34645 OnSpellCast50percent Talk2 acceptance pending implementation"]
fn encounter_coverage_gargolmar_speech_surge_casts_have_both_source_chance_outcomes() {
    let (mut h, boss, _) = super::gargolmar::pull_gargolmar();
    let player = h.player();
    let mut actual_casts = 0;
    let mut spoken = 0;
    // Source has no mana cost or category recovery; keep ordinary admission and
    // real AI clocks. Scheduled Surge casts count too, rather than resetting AI.
    for trial in 0..64 {
        let casts_before = spell_gos(&h, boss, 34645).len();
        let speech_before = authored_yells(&h, boss, 2);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, 34645, Some(player), false)
            .unwrap_or_else(|failure| panic!("native ordinary Surge trial{trial}: {failure:?}"));
        h.advance(1.1);
        let casts = spell_gos(&h, boss, 34645).len() - casts_before;
        let speech = authored_yells(&h, boss, 2) - speech_before;
        assert!(casts >= 1, "actual ordinary Surge SpellGo for trial{trial}");
        assert!(
            speech <= casts,
            "at most one source yell per successful cast"
        );
        actual_casts += casts;
        spoken += speech;
        assert!(
            h.health(player).current > 0.0,
            "native encounter buffer stays alive"
        );
        assert!(
            h.inbox(1).chat.is_empty(),
            "other copy cannot hear Surge speech"
        );
    }
    assert!(
        h.inbox(0).log.iter().any(|event| {
            event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(34645)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }),
        "actual source Surge impact occurs; misses still retain successful-cast hooks"
    );
    assert!(
        spoken > 0,
        "source34645 roll50percent must sometimes emit actual Talk2"
    );
    assert!(
        spoken < actual_casts,
        "source50percent branch must sometimes remain quiet"
    );
}
