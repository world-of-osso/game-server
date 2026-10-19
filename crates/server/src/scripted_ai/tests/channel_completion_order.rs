//! TC a352 Spell.cpp4342–4350 vs cancel3642: natural finish before forced aura cleanup.
use super::*;

const STOMP: u32 = 374720;
const DAMAGE: u32 = 374731;

fn player_event_index(
    h: &Harness,
    leymor: Entity,
    kind: CombatLogKind,
    spell: u32,
) -> Option<usize> {
    h.inbox(0).log.iter().position(|event| {
        event.source == Some(leymor.to_bits())
            && event.target == Some(h.player().to_bits())
            && event.spell_id == Some(spell)
            && event.kind == kind
    })
}

#[test]
#[ignore = "development RED: successful-channel lifecycle ordering"]
fn channel_completion_order_native_finish_damage_precedes_actual_recipient_aura_removal() {
    let (mut h, leymor) = super::azure_stomp_removal::fixture();
    let recipients = super::azure_stomp_removal::wait_for_stomp(&mut h, leymor);
    let player = h.player();
    assert!(
        h.has_aura(player, STOMP),
        "actual Player e1 aura before finish"
    );
    assert!(
        !recipients.is_empty(),
        "actual native e0 recipients also present"
    );
    h.players[0].2.log.clear();
    let cast = h.app.world().get::<CastState>(leymor).unwrap();
    let remaining = cast.duration - cast.elapsed;
    h.advance(remaining + 0.2);
    let damage = player_event_index(&h, leymor, CombatLogKind::Damage, DAMAGE)
        .expect("actual ordinary OnChannelFinished damage, not a manual child");
    let removal = player_event_index(&h, leymor, CombatLogKind::AuraRemoved, STOMP)
        .expect("actual native recipient expiry/removal is observed on the Player wire");
    eprintln!(
        "native channel order: finish_damage_index={damage}, player_aura_removal_index={removal}"
    );
    assert!(
        damage < removal,
        "successful finish callback/damage must precede forced recipient cleanup, not cancel-style removal"
    );
    assert!(
        !h.has_aura(player, STOMP),
        "recipient still expires naturally"
    );
    for recipient in recipients {
        assert!(!crate::spell_cast::targets::is_alive(
            h.app.world(),
            recipient
        ));
        assert!(
            h.app
                .world()
                .get::<crate::combat::Corpse>(recipient)
                .is_some()
        );
    }
}

#[test]
#[ignore = "development RED control: cancellation must remain immediate"]
fn channel_completion_order_native_cancel_removes_recipients_before_return_without_finish_damage() {
    let (mut h, leymor) = super::azure_stomp_removal::fixture();
    let recipients = super::azure_stomp_removal::wait_for_stomp(&mut h, leymor);
    let player = h.player();
    let health = h.health(player).current;
    h.players[0].2.log.clear();
    crate::spell_cast::cancel_cast(h.app.world_mut(), leymor);
    // Observe the actual cancel API boundary before another world tick.
    assert!(h.app.world().get::<CastState>(leymor).is_none());
    assert!(!h.has_aura(player, STOMP));
    assert_eq!(
        h.health(player).current,
        health,
        "cancel never invokes finish damage"
    );
    for recipient in recipients {
        assert!(!h.has_aura(recipient, STOMP));
        assert!(
            !crate::spell_cast::targets::is_alive(h.app.world(), recipient),
            "cancel removes actual e0 auras and runs self-death before returning"
        );
    }
    h.advance(0.1);
    assert!(player_event_index(&h, leymor, CombatLogKind::AuraRemoved, STOMP).is_some());
    assert!(player_event_index(&h, leymor, CombatLogKind::Damage, DAMAGE).is_none());
}
