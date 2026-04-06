use super::*;

#[test]
fn default_motd_is_empty() {
    let motd = Motd::default();
    assert!(!motd.is_set());
    assert!(motd.message.is_empty());
}

#[test]
fn set_motd() {
    let mut motd = Motd::default();
    motd.set("Welcome to World of Osso!", 1000).unwrap();
    assert!(motd.is_set());
    assert_eq!(motd.message, "Welcome to World of Osso!");
    assert_eq!(motd.updated_at, 1000);
}

#[test]
fn set_motd_replaces_previous() {
    let mut motd = Motd::default();
    motd.set("First message", 1000).unwrap();
    motd.set("Updated message", 2000).unwrap();
    assert_eq!(motd.message, "Updated message");
    assert_eq!(motd.updated_at, 2000);
}

#[test]
fn set_motd_too_long() {
    let mut motd = Motd::default();
    let long = "x".repeat(MAX_MOTD_LEN + 1);
    assert_eq!(motd.set(&long, 1000), Err(MotdError::TooLong));
    assert!(!motd.is_set());
}

#[test]
fn set_motd_max_length_ok() {
    let mut motd = Motd::default();
    let max = "x".repeat(MAX_MOTD_LEN);
    assert!(motd.set(&max, 1000).is_ok());
}

#[test]
fn clear_motd() {
    let mut motd = Motd::default();
    motd.set("Hello", 1000).unwrap();
    motd.clear(2000);
    assert!(!motd.is_set());
    assert_eq!(motd.updated_at, 2000);
}

#[test]
fn set_empty_string() {
    let mut motd = Motd::default();
    motd.set("", 1000).unwrap();
    assert!(!motd.is_set());
}

// --- Autobroadcast ---

#[test]
fn autobroadcast_default_disabled() {
    let ab = Autobroadcast::new();
    assert!(!ab.enabled);
    assert!(ab.messages.is_empty());
    assert_eq!(ab.interval, DEFAULT_BROADCAST_INTERVAL);
}

#[test]
fn autobroadcast_disabled_never_fires() {
    let mut ab = Autobroadcast::new();
    ab.add_message("Hello".to_string());
    // Not enabled
    assert!(ab.should_broadcast(10000).is_none());
}

#[test]
fn autobroadcast_empty_never_fires() {
    let mut ab = Autobroadcast::new();
    ab.enabled = true;
    // No messages
    assert!(ab.should_broadcast(10000).is_none());
}

#[test]
fn autobroadcast_fires_after_interval() {
    let mut ab = Autobroadcast::new();
    ab.add_message("Server restarting soon!".to_string());
    ab.enabled = true;
    ab.interval = 60;

    // First call: fires immediately (last_broadcast=0, now=60)
    let msg = ab.should_broadcast(60).unwrap();
    assert_eq!(msg, "Server restarting soon!");

    // Too early
    assert!(ab.should_broadcast(100).is_none());

    // After interval
    let msg = ab.should_broadcast(120).unwrap();
    assert_eq!(msg, "Server restarting soon!");
}

#[test]
fn autobroadcast_cycles_messages() {
    let mut ab = Autobroadcast::new();
    ab.add_message("First".to_string());
    ab.add_message("Second".to_string());
    ab.add_message("Third".to_string());
    ab.enabled = true;
    ab.interval = 10;

    assert_eq!(ab.should_broadcast(10).unwrap(), "First");
    assert_eq!(ab.should_broadcast(20).unwrap(), "Second");
    assert_eq!(ab.should_broadcast(30).unwrap(), "Third");
    // Wraps around
    assert_eq!(ab.should_broadcast(40).unwrap(), "First");
}

#[test]
fn autobroadcast_add_remove() {
    let mut ab = Autobroadcast::new();
    ab.add_message("A".to_string());
    ab.add_message("B".to_string());
    ab.add_message("C".to_string());
    assert_eq!(ab.message_count(), 3);

    let removed = ab.remove_message(1).unwrap();
    assert_eq!(removed, "B");
    assert_eq!(ab.message_count(), 2);
    assert_eq!(ab.messages[0], "A");
    assert_eq!(ab.messages[1], "C");
}

#[test]
fn autobroadcast_remove_out_of_bounds() {
    let mut ab = Autobroadcast::new();
    assert!(ab.remove_message(0).is_none());
}

#[test]
fn autobroadcast_remove_resets_index() {
    let mut ab = Autobroadcast::new();
    ab.add_message("A".to_string());
    ab.add_message("B".to_string());
    ab.enabled = true;
    ab.interval = 10;

    // Advance to index 1
    ab.should_broadcast(10);
    // Remove "B" (index 1) — next_index was 1, now only 1 message, wraps to 0
    ab.remove_message(1);
    assert_eq!(ab.should_broadcast(20).unwrap(), "A");
}

// --- Chat integration ---

use crate::protocol::ChatType;

#[test]
fn motd_as_chat_message() {
    let mut motd = Motd::default();
    motd.set("Welcome!", 1000).unwrap();

    let msg = motd.as_chat_message().unwrap();
    assert_eq!(msg.sender, SYSTEM_SENDER);
    assert_eq!(msg.content, "Welcome!");
    assert_eq!(msg.channel, ChatType::System);
}

#[test]
fn motd_empty_no_chat_message() {
    let motd = Motd::default();
    assert!(motd.as_chat_message().is_none());
}

#[test]
fn autobroadcast_tick_chat_message() {
    let mut ab = Autobroadcast::new();
    ab.add_message("Reminder!".to_string());
    ab.enabled = true;
    ab.interval = 60;

    let msg = ab.tick_chat_message(60).unwrap();
    assert_eq!(msg.sender, SYSTEM_SENDER);
    assert_eq!(msg.content, "Reminder!");
    assert_eq!(msg.channel, ChatType::ServerBroadcast);
}

#[test]
fn autobroadcast_tick_not_due() {
    let mut ab = Autobroadcast::new();
    ab.add_message("Reminder!".to_string());
    ab.enabled = true;
    ab.interval = 60;

    ab.tick_chat_message(60); // fires
    assert!(ab.tick_chat_message(90).is_none()); // not due
}

#[test]
fn autobroadcast_tick_disabled() {
    let mut ab = Autobroadcast::new();
    ab.add_message("Reminder!".to_string());
    ab.interval = 60;
    // Not enabled
    assert!(ab.tick_chat_message(60).is_none());
}
