use super::*;
use crate::spell_cast::test_harness::Harness;
use shared::protocol::SetWarModeDesired;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
enum SelectionCase {
    Fresh,
    Departed,
    QueuedCooldown,
    WithWarMode,
}

#[test]
fn pvp_talent_selection_waits_for_fifo_preserves_honor_and_gates_spell_grants() {
    assert_selection_fifo(SelectionCase::Fresh);
}

#[test]
fn pvp_talent_departure_withholds_live_selection_grants_and_reply_after_commit() {
    assert_selection_fifo(SelectionCase::Departed);
}

#[test]
fn pvp_talent_second_queued_selection_checks_the_first_selected_spells_real_cooldown() {
    assert_selection_fifo(SelectionCase::QueuedCooldown);
}

#[test]
fn pvp_talent_then_war_mode_preserve_both_durable_and_live_preferences() {
    assert_selection_fifo(SelectionCase::WithWarMode);
}

fn selection_harness(case: SelectionCase) -> (Harness, u64) {
    let (mut h, character) =
        crate::pvp::pvp_war_mode::city_harness_with_spells(&[282559, 269083, 210256, 204018]);
    h.app.insert_resource(Catalog::fixture());
    let baseline = if matches!(case, SelectionCase::QueuedCooldown) {
        vec![204018]
    } else {
        vec![]
    };
    h.app.world_mut().entity_mut(h.caster).insert((
        ActiveSpec(70),
        Level(90),
        KnownSpells::fixture(baseline),
    ));
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<SelectPvpTalent>::default(),
        MessageReceiver::<SelectPvpTalent>::default(),
        MessageSender::<SpellsLearned>::default(),
        MessageReceiver::<SpellsLearned>::default(),
        MessageSender::<SpellsUnlearned>::default(),
        MessageReceiver::<SpellsUnlearned>::default(),
    ));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 282559,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(project(h.app.world(), h.caster).unwrap().1.is_empty());
    h.app.add_systems(
        Update,
        receive_requests.before(crate::pvp::pvp_war_mode::receive_requests),
    );
    h.app.add_systems(PostUpdate, sync_players);
    h.advance(6);
    if matches!(case, SelectionCase::QueuedCooldown) {
        h.cast_and_deliver(204018, Some(h.caster));
        h.advance(3);
        let world = h.app.world();
        let spell = world.resource::<SpellStore>().get(204018).unwrap();
        let now = world.resource::<Time<Virtual>>().elapsed_secs_f64();
        assert!(
            world
                .get::<crate::spell_cast::SpellHistory>(h.caster)
                .unwrap()
                .on_cooldown(spell, now),
            "real Blessing of Sanctuary cast must establish the removal cooldown"
        );
    }
    h.app
        .world_mut()
        .get_mut::<MessageReceiver<PvpStateUpdate>>(h.connection)
        .unwrap()
        .receive()
        .for_each(drop);
    (h, character)
}

fn send_selection(h: &mut Harness, id: u32) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<SelectPvpTalent>>(h.connection)
        .unwrap()
        .send::<PvpChannel>(SelectPvpTalent {
            slot: 1,
            talent_id: id,
        });
}

fn assert_selection_fifo(case: SelectionCase) {
    let (mut h, character) = selection_harness(case);
    let db = h.app.world().resource::<GameDatabase>();
    let held = db.db.begin_write().unwrap();
    let prior = db.writer.enqueue("earlier PvP talent honor", move |db| {
        let mut progress = persistence::load_pvp(db, character);
        progress.honor = 777;
        persistence::save_pvp(db, character, &progress);
        Ok(())
    });
    let release = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(250));
        held.abort().unwrap();
    });
    if matches!(case, SelectionCase::QueuedCooldown) {
        send_selection(&mut h, 5573);
    }
    send_selection(&mut h, 752);
    if matches!(case, SelectionCase::WithWarMode) {
        h.app
            .world_mut()
            .get_mut::<MessageSender<SetWarModeDesired>>(h.connection)
            .unwrap()
            .send::<PvpChannel>(SetWarModeDesired { enabled: true });
    }
    let started = Instant::now();
    h.advance(3);
    let elapsed = started.elapsed();
    let premature: Vec<_> = h
        .app
        .world_mut()
        .get_mut::<MessageReceiver<PvpStateUpdate>>(h.connection)
        .unwrap()
        .receive()
        .collect();
    let premature_selected = h
        .app
        .world()
        .get::<PlayerPvpState>(h.caster)
        .unwrap()
        .progress
        .pvp_talents
        .get(&70)
        .cloned()
        .unwrap_or_default();
    let premature_grant = h
        .app
        .world()
        .get::<KnownSpells>(h.caster)
        .unwrap()
        .contains(210256);
    if matches!(case, SelectionCase::Departed) {
        h.app
            .world_mut()
            .entity_mut(h.connection)
            .remove::<InWorld>();
    }
    let fence = h
        .app
        .world()
        .resource::<GameDatabase>()
        .writer
        .enqueue("PvP selection fence", |_| Ok(()));
    release.join().unwrap();
    assert!(
        elapsed < Duration::from_millis(50),
        "PvP talent tick waited: {elapsed:?}"
    );
    assert!(
        premature.is_empty(),
        "talent reply preceded its durable selection"
    );
    assert!(
        premature_selected.is_empty() && !premature_grant,
        "talent live effects preceded commit"
    );
    let expected = if matches!(case, SelectionCase::QueuedCooldown) {
        5573
    } else {
        752
    };
    let replies = await_selection(&mut h, case, expected, fence);
    assert_eq!(prior.result(), Some(Ok(())));
    let durable = persistence::load_pvp(&h.app.world().resource::<GameDatabase>().db, character);
    assert_eq!(durable.honor, 777);
    assert_eq!(durable.pvp_talents[&70][&1], expected);
    assert_eq!(
        durable.war_mode_desired,
        matches!(case, SelectionCase::WithWarMode)
    );
    let state = h.app.world().get::<PlayerPvpState>(h.caster).unwrap();
    if matches!(case, SelectionCase::Departed) {
        assert!(state.progress.pvp_talents.is_empty());
        assert!(replies.is_empty());
    } else {
        assert_eq!(state.progress.honor, 777);
        assert_eq!(state.progress.pvp_talents[&70][&1], expected);
        assert_eq!(state.progress.war_mode_desired, durable.war_mode_desired);
        let last = replies.last().unwrap();
        if matches!(case, SelectionCase::QueuedCooldown) {
            assert!(last.error.as_ref().unwrap().contains("cooldown"));
        } else {
            assert_eq!(last.error, None);
        }
    }
    assert_eq!(
        h.app
            .world()
            .get::<KnownSpells>(h.caster)
            .unwrap()
            .contains(210256),
        !matches!(
            case,
            SelectionCase::Departed | SelectionCase::QueuedCooldown
        )
    );
}

fn await_selection(
    h: &mut Harness,
    case: SelectionCase,
    expected: u32,
    fence: crate::persistence_writer::CommitReceipt,
) -> Vec<PvpStateUpdate> {
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut replies = Vec::new();
    let mut frames = 0;
    loop {
        h.advance(1);
        frames += 1;
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<PvpStateUpdate>>(h.connection)
                .unwrap()
                .receive(),
        );
        let selected_reply = replies.last().is_some_and(|reply: &PvpStateUpdate| {
            let Some(snapshot) = &reply.snapshot else {
                return false;
            };
            let selected = snapshot.talents.as_ref().is_some_and(|talents| {
                talents
                    .slots
                    .iter()
                    .any(|slot| slot.index == 1 && slot.selected == Some(expected))
            });
            let mode_ok = !matches!(case, SelectionCase::WithWarMode)
                || snapshot.war_mode.as_ref().is_some_and(|mode| mode.desired);
            let error_ok = !matches!(case, SelectionCase::QueuedCooldown) || reply.error.is_some();
            selected && mode_ok && error_ok
        });
        let replied = matches!(case, SelectionCase::Departed) || selected_reply;
        if frames >= 6 && fence.result().is_some() && replied {
            return replies;
        }
        assert!(
            Instant::now() < deadline,
            "committed PvP selection reply timed out: {replies:?}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}
