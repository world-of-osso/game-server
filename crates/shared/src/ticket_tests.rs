use super::*;

#[test]
fn create_ticket() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "NPCs won't talk to me", 1000).unwrap();

    assert_eq!(id, 1);
    assert_eq!(mgr.len(), 1);

    let ticket = mgr.get(id).unwrap();
    assert_eq!(ticket.player, 100);
    assert_eq!(ticket.category, TicketCategory::Bug);
    assert_eq!(ticket.description, "NPCs won't talk to me");
    assert_eq!(ticket.state, TicketState::Open);
    assert_eq!(ticket.created_at, 1000);
}

#[test]
fn create_sequential_ids() {
    let mut mgr = TicketManager::new();
    let id1 = mgr.create(100, TicketCategory::Bug, "Issue 1", 1000).unwrap();
    let id2 = mgr.create(200, TicketCategory::Quest, "Issue 2", 1001).unwrap();
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn one_ticket_per_player() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "First issue", 1000).unwrap();

    assert_eq!(
        mgr.create(100, TicketCategory::Quest, "Second issue", 1001),
        Err(TicketError::AlreadyHasTicket)
    );
}

#[test]
fn empty_description_rejected() {
    let mut mgr = TicketManager::new();
    assert_eq!(
        mgr.create(100, TicketCategory::Bug, "", 1000),
        Err(TicketError::EmptyDescription)
    );
}

#[test]
fn long_description_rejected() {
    let mut mgr = TicketManager::new();
    let long = "x".repeat(MAX_DESCRIPTION_LEN + 1);
    assert_eq!(
        mgr.create(100, TicketCategory::Bug, &long, 1000),
        Err(TicketError::DescriptionTooLong)
    );
}

#[test]
fn max_length_description_accepted() {
    let mut mgr = TicketManager::new();
    let max_len = "x".repeat(MAX_DESCRIPTION_LEN);
    assert!(mgr.create(100, TicketCategory::Bug, &max_len, 1000).is_ok());
}

#[test]
fn get_nonexistent() {
    let mgr = TicketManager::new();
    assert!(mgr.get(999).is_none());
}

#[test]
fn find_player_ticket() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Stuck, "I'm stuck", 1000).unwrap();

    let ticket = mgr.find_player_ticket(100).unwrap();
    assert_eq!(ticket.id, id);

    assert!(mgr.find_player_ticket(200).is_none());
}

#[test]
fn open_tickets_list() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "Bug report", 1000).unwrap();
    mgr.create(200, TicketCategory::Harassment, "Player harassment", 1001).unwrap();

    let open = mgr.open_tickets();
    assert_eq!(open.len(), 2);
}

#[test]
fn all_categories_accepted() {
    let mut mgr = TicketManager::new();
    let categories = [
        TicketCategory::Stuck,
        TicketCategory::Bug,
        TicketCategory::Harassment,
        TicketCategory::Item,
        TicketCategory::Quest,
        TicketCategory::Account,
        TicketCategory::Other,
    ];
    for (i, cat) in categories.iter().enumerate() {
        let player = 100 + i as u64;
        mgr.create(player, *cat, "Test ticket", 1000 + i as u64).unwrap();
    }
    assert_eq!(mgr.len(), 7);
}

#[test]
fn different_players_can_each_have_ticket() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "Player 100", 1000).unwrap();
    mgr.create(200, TicketCategory::Bug, "Player 200", 1001).unwrap();
    mgr.create(300, TicketCategory::Bug, "Player 300", 1002).unwrap();
    assert_eq!(mgr.len(), 3);
}
