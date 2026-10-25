use super::{MessageReceiver, MessageSender};
use std::any::TypeId;

struct CombatChannel;
struct ChatChannel;

#[test]
fn sending_preserves_payload_order_channels_and_priorities() {
    let mut sender = MessageSender::<String>::default();
    sender.send::<CombatChannel>("cast 133".into());
    sender.send_with_priority::<ChatChannel>("ready".into(), 0.25);
    sender.send::<CombatChannel>("cancel 133".into());
    let messages: Vec<_> = sender
        .drain()
        .map(|message| (message.message, message.channel, message.priority))
        .collect();
    assert_eq!(
        messages,
        vec![
            ("cast 133".into(), TypeId::of::<CombatChannel>(), 1.0),
            ("ready".into(), TypeId::of::<ChatChannel>(), 0.25),
            ("cancel 133".into(), TypeId::of::<CombatChannel>(), 1.0),
        ]
    );
    assert_eq!(sender.drain().count(), 0);
}

#[test]
fn receiving_drains_payloads_and_clear_discards_unread_messages() {
    let mut receiver = MessageReceiver::<u32>::default();
    receiver.push(133);
    receiver.push(17);
    assert!(receiver.has_messages());
    assert_eq!(receiver.receive().collect::<Vec<_>>(), vec![133, 17]);
    assert!(!receiver.has_messages());
    receiver.push(116);
    receiver.clear();
    assert_eq!(receiver.receive().count(), 0);
}
