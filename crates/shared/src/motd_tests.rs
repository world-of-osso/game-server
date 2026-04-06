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
