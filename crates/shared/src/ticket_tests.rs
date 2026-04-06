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

// --- Priority and queue ---

#[test]
fn default_priorities() {
    assert_eq!(default_priority(TicketCategory::Harassment), TicketPriority::Critical);
    assert_eq!(default_priority(TicketCategory::Stuck), TicketPriority::High);
    assert_eq!(default_priority(TicketCategory::Bug), TicketPriority::Normal);
    assert_eq!(default_priority(TicketCategory::Quest), TicketPriority::Normal);
    assert_eq!(default_priority(TicketCategory::Other), TicketPriority::Normal);
}

#[test]
fn priority_ordering() {
    assert!(TicketPriority::Critical > TicketPriority::High);
    assert!(TicketPriority::High > TicketPriority::Normal);
}

#[test]
fn queue_orders_by_priority_then_fifo() {
    let mut mgr = TicketManager::new();
    // Normal priority, oldest
    mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    // Critical priority, newer
    mgr.create(200, TicketCategory::Harassment, "Harass", 1001).unwrap();
    // High priority, newest
    mgr.create(300, TicketCategory::Stuck, "Stuck", 1002).unwrap();

    let queue = mgr.queue();
    assert_eq!(queue.len(), 3);
    // Critical first
    assert_eq!(queue[0].player, 200);
    assert_eq!(queue[0].priority, TicketPriority::Critical);
    // High second
    assert_eq!(queue[1].player, 300);
    assert_eq!(queue[1].priority, TicketPriority::High);
    // Normal last
    assert_eq!(queue[2].player, 100);
    assert_eq!(queue[2].priority, TicketPriority::Normal);
}

#[test]
fn queue_fifo_within_same_priority() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "First bug", 1000).unwrap();
    mgr.create(200, TicketCategory::Quest, "Quest issue", 1001).unwrap();
    mgr.create(300, TicketCategory::Item, "Item problem", 1002).unwrap();

    let queue = mgr.queue();
    // All Normal priority — should be FIFO (oldest first)
    assert_eq!(queue[0].player, 100);
    assert_eq!(queue[1].player, 200);
    assert_eq!(queue[2].player, 300);
}

#[test]
fn next_in_queue_returns_highest_priority_oldest() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.create(200, TicketCategory::Stuck, "Stuck", 1001).unwrap();

    let next = mgr.next_in_queue().unwrap();
    assert_eq!(next.player, 200); // High priority
}

#[test]
fn next_in_queue_empty() {
    let mgr = TicketManager::new();
    assert!(mgr.next_in_queue().is_none());
}

#[test]
fn set_priority_override() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    assert_eq!(mgr.get(id).unwrap().priority, TicketPriority::Normal);

    mgr.set_priority(id, TicketPriority::Critical).unwrap();
    assert_eq!(mgr.get(id).unwrap().priority, TicketPriority::Critical);
}

#[test]
fn set_priority_not_found() {
    let mut mgr = TicketManager::new();
    assert_eq!(mgr.set_priority(999, TicketPriority::High), Err(TicketError::NotFound));
}

#[test]
fn queue_counts() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.create(200, TicketCategory::Quest, "Quest", 1001).unwrap();
    mgr.create(300, TicketCategory::Stuck, "Stuck", 1002).unwrap();
    mgr.create(400, TicketCategory::Harassment, "Harass", 1003).unwrap();

    let (critical, high, normal) = mgr.queue_counts();
    assert_eq!(critical, 1);
    assert_eq!(high, 1);
    assert_eq!(normal, 2);
}

#[test]
fn priority_override_affects_queue_order() {
    let mut mgr = TicketManager::new();
    let bug_id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.create(200, TicketCategory::Quest, "Quest", 1001).unwrap();

    // Elevate the bug to Critical
    mgr.set_priority(bug_id, TicketPriority::Critical).unwrap();

    let next = mgr.next_in_queue().unwrap();
    assert_eq!(next.id, bug_id);
    assert_eq!(next.priority, TicketPriority::Critical);
}
