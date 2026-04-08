use super::*;

#[test]
fn create_ticket() {
    let mut mgr = TicketManager::new();
    let id = mgr
        .create(100, TicketCategory::Bug, "NPCs won't talk to me", 1000)
        .unwrap();

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
    let id1 = mgr
        .create(100, TicketCategory::Bug, "Issue 1", 1000)
        .unwrap();
    let id2 = mgr
        .create(200, TicketCategory::Quest, "Issue 2", 1001)
        .unwrap();
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn one_ticket_per_player() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "First issue", 1000)
        .unwrap();

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
    let id = mgr
        .create(100, TicketCategory::Stuck, "I'm stuck", 1000)
        .unwrap();

    let ticket = mgr.find_player_ticket(100).unwrap();
    assert_eq!(ticket.id, id);

    assert!(mgr.find_player_ticket(200).is_none());
}

#[test]
fn open_tickets_list() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "Bug report", 1000)
        .unwrap();
    mgr.create(200, TicketCategory::Harassment, "Player harassment", 1001)
        .unwrap();

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
        mgr.create(player, *cat, "Test ticket", 1000 + i as u64)
            .unwrap();
    }
    assert_eq!(mgr.len(), 7);
}

#[test]
fn different_players_can_each_have_ticket() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "Player 100", 1000)
        .unwrap();
    mgr.create(200, TicketCategory::Bug, "Player 200", 1001)
        .unwrap();
    mgr.create(300, TicketCategory::Bug, "Player 300", 1002)
        .unwrap();
    assert_eq!(mgr.len(), 3);
}

// --- Priority and queue ---

#[test]
fn default_priorities() {
    assert_eq!(
        default_priority(TicketCategory::Harassment),
        TicketPriority::Critical
    );
    assert_eq!(
        default_priority(TicketCategory::Stuck),
        TicketPriority::High
    );
    assert_eq!(
        default_priority(TicketCategory::Bug),
        TicketPriority::Normal
    );
    assert_eq!(
        default_priority(TicketCategory::Quest),
        TicketPriority::Normal
    );
    assert_eq!(
        default_priority(TicketCategory::Other),
        TicketPriority::Normal
    );
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
    mgr.create(200, TicketCategory::Harassment, "Harass", 1001)
        .unwrap();
    // High priority, newest
    mgr.create(300, TicketCategory::Stuck, "Stuck", 1002)
        .unwrap();

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
    mgr.create(100, TicketCategory::Bug, "First bug", 1000)
        .unwrap();
    mgr.create(200, TicketCategory::Quest, "Quest issue", 1001)
        .unwrap();
    mgr.create(300, TicketCategory::Item, "Item problem", 1002)
        .unwrap();

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
    mgr.create(200, TicketCategory::Stuck, "Stuck", 1001)
        .unwrap();

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
    assert_eq!(
        mgr.set_priority(999, TicketPriority::High),
        Err(TicketError::NotFound)
    );
}

#[test]
fn queue_counts() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.create(200, TicketCategory::Quest, "Quest", 1001)
        .unwrap();
    mgr.create(300, TicketCategory::Stuck, "Stuck", 1002)
        .unwrap();
    mgr.create(400, TicketCategory::Harassment, "Harass", 1003)
        .unwrap();

    let (critical, high, normal) = mgr.queue_counts();
    assert_eq!(critical, 1);
    assert_eq!(high, 1);
    assert_eq!(normal, 2);
}

#[test]
fn priority_override_affects_queue_order() {
    let mut mgr = TicketManager::new();
    let bug_id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.create(200, TicketCategory::Quest, "Quest", 1001)
        .unwrap();

    // Elevate the bug to Critical
    mgr.set_priority(bug_id, TicketPriority::Critical).unwrap();

    let next = mgr.next_in_queue().unwrap();
    assert_eq!(next.id, bug_id);
    assert_eq!(next.priority, TicketPriority::Critical);
}

// --- GM assignment ---

#[test]
fn assign_ticket() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();

    mgr.assign(id, "GM_Alice").unwrap();
    let ticket = mgr.get(id).unwrap();
    assert_eq!(ticket.state, TicketState::Assigned);
    assert_eq!(ticket.assigned_to.as_deref(), Some("GM_Alice"));
}

#[test]
fn assign_removes_from_queue() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.create(200, TicketCategory::Quest, "Quest", 1001)
        .unwrap();

    mgr.assign(id, "GM_Alice").unwrap();
    // Assigned ticket should not appear in the open queue
    let queue = mgr.queue();
    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].player, 200);
}

#[test]
fn assign_already_assigned_fails() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();

    assert_eq!(mgr.assign(id, "GM_Bob"), Err(TicketError::NotOpen));
}

#[test]
fn assign_not_found() {
    let mut mgr = TicketManager::new();
    assert_eq!(mgr.assign(999, "GM_Alice"), Err(TicketError::NotFound));
}

#[test]
fn unassign_returns_to_queue() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.unassign(id).unwrap();

    let ticket = mgr.get(id).unwrap();
    assert_eq!(ticket.state, TicketState::Open);
    assert!(ticket.assigned_to.is_none());

    // Back in queue
    assert_eq!(mgr.queue().len(), 1);
}

#[test]
fn unassign_not_assigned_fails() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    assert_eq!(mgr.unassign(id), Err(TicketError::NotAssigned));
}

#[test]
fn claim_next_assigns_highest_priority() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    let stuck_id = mgr
        .create(200, TicketCategory::Stuck, "Stuck", 1001)
        .unwrap();

    let claimed = mgr.claim_next("GM_Alice").unwrap();
    assert_eq!(claimed, stuck_id); // High priority
    assert_eq!(
        mgr.get(stuck_id).unwrap().assigned_to.as_deref(),
        Some("GM_Alice")
    );
}

#[test]
fn claim_next_empty_queue_fails() {
    let mut mgr = TicketManager::new();
    assert_eq!(mgr.claim_next("GM_Alice"), Err(TicketError::NotFound));
}

#[test]
fn assigned_to_lists_gm_tickets() {
    let mut mgr = TicketManager::new();
    let id1 = mgr.create(100, TicketCategory::Bug, "Bug 1", 1000).unwrap();
    let id2 = mgr.create(200, TicketCategory::Bug, "Bug 2", 1001).unwrap();
    mgr.create(300, TicketCategory::Bug, "Bug 3", 1002).unwrap();

    mgr.assign(id1, "GM_Alice").unwrap();
    mgr.assign(id2, "GM_Alice").unwrap();

    let alice_tickets = mgr.assigned_to("GM_Alice");
    assert_eq!(alice_tickets.len(), 2);
    assert!(mgr.assigned_to("GM_Bob").is_empty());
}

// --- State transitions: resolve / escalate ---

#[test]
fn resolve_assigned_ticket() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.resolve(id).unwrap();

    assert_eq!(mgr.get(id).unwrap().state, TicketState::Resolved);
}

#[test]
fn resolve_open_ticket_fails() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    assert_eq!(mgr.resolve(id), Err(TicketError::InvalidTransition));
}

#[test]
fn resolve_already_resolved_fails() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.resolve(id).unwrap();
    assert_eq!(mgr.resolve(id), Err(TicketError::AlreadyResolved));
}

#[test]
fn resolve_escalated_ticket() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.escalate(id).unwrap();
    mgr.resolve(id).unwrap();

    assert_eq!(mgr.get(id).unwrap().state, TicketState::Resolved);
}

#[test]
fn escalate_assigned_ticket() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.escalate(id).unwrap();

    assert_eq!(mgr.get(id).unwrap().state, TicketState::Escalated);
}

#[test]
fn escalate_open_ticket_fails() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    assert_eq!(mgr.escalate(id), Err(TicketError::InvalidTransition));
}

#[test]
fn escalate_resolved_ticket_fails() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.resolve(id).unwrap();
    assert_eq!(mgr.escalate(id), Err(TicketError::AlreadyResolved));
}

#[test]
fn resolved_ticket_frees_player() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.resolve(id).unwrap();

    // Player can create a new ticket after resolution
    mgr.create(100, TicketCategory::Quest, "New issue", 2000)
        .unwrap();
    assert_eq!(mgr.len(), 2);
}

#[test]
fn resolved_tickets_list() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.resolve(id).unwrap();

    assert_eq!(mgr.resolved_tickets().len(), 1);
    assert!(mgr.open_tickets().is_empty());
}

#[test]
fn escalated_tickets_list() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.escalate(id).unwrap();

    assert_eq!(mgr.escalated_tickets().len(), 1);
}

#[test]
fn full_lifecycle_open_assigned_resolved() {
    let mut mgr = TicketManager::new();
    let id = mgr
        .create(100, TicketCategory::Stuck, "I'm stuck", 1000)
        .unwrap();
    assert_eq!(mgr.get(id).unwrap().state, TicketState::Open);

    mgr.assign(id, "GM_Alice").unwrap();
    assert_eq!(mgr.get(id).unwrap().state, TicketState::Assigned);

    mgr.resolve(id).unwrap();
    assert_eq!(mgr.get(id).unwrap().state, TicketState::Resolved);
}

#[test]
fn full_lifecycle_open_assigned_escalated_resolved() {
    let mut mgr = TicketManager::new();
    let id = mgr
        .create(100, TicketCategory::Harassment, "Harass", 1000)
        .unwrap();

    mgr.assign(id, "GM_Alice").unwrap();
    mgr.escalate(id).unwrap();
    assert_eq!(mgr.get(id).unwrap().state, TicketState::Escalated);

    mgr.resolve(id).unwrap();
    assert_eq!(mgr.get(id).unwrap().state, TicketState::Resolved);
}

// --- Player notifications ---

#[test]
fn assign_notifies_player() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();

    let notifs = mgr.drain_notifications();
    assert_eq!(notifs.len(), 1);
    assert!(matches!(&notifs[0],
        TicketNotification::Assigned { ticket_id, player, gm_name }
        if *ticket_id == id && *player == 100 && gm_name == "GM_Alice"
    ));
}

#[test]
fn resolve_notifies_player() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.drain_notifications();

    mgr.resolve(id).unwrap();
    let notifs = mgr.drain_notifications();
    assert_eq!(notifs.len(), 1);
    assert!(matches!(&notifs[0],
        TicketNotification::Resolved { ticket_id, player }
        if *ticket_id == id && *player == 100
    ));
}

#[test]
fn escalate_notifies_player() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.drain_notifications();

    mgr.escalate(id).unwrap();
    let notifs = mgr.drain_notifications();
    assert_eq!(notifs.len(), 1);
    assert!(matches!(&notifs[0],
        TicketNotification::Escalated { ticket_id, player }
        if *ticket_id == id && *player == 100
    ));
}

#[test]
fn gm_respond_notifies_player() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.drain_notifications();

    mgr.respond(id, "GM_Alice", "Working on your issue!", 2000)
        .unwrap();

    let notifs = mgr.drain_notifications();
    assert_eq!(notifs.len(), 1);
    assert!(matches!(&notifs[0],
        TicketNotification::GmResponse { ticket_id, player, gm_name, message }
        if *ticket_id == id && *player == 100 && gm_name == "GM_Alice"
           && message == "Working on your issue!"
    ));
}

#[test]
fn respond_stores_on_ticket() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();

    mgr.respond(id, "GM_Alice", "First response", 2000).unwrap();
    mgr.respond(id, "GM_Alice", "Follow-up", 3000).unwrap();

    let ticket = mgr.get(id).unwrap();
    assert_eq!(ticket.responses.len(), 2);
    assert_eq!(ticket.responses[0].message, "First response");
    assert_eq!(ticket.responses[1].message, "Follow-up");
    assert_eq!(ticket.responses[0].timestamp, 2000);
}

#[test]
fn respond_to_resolved_fails() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.resolve(id).unwrap();

    assert_eq!(
        mgr.respond(id, "GM_Alice", "Late reply", 5000),
        Err(TicketError::AlreadyResolved)
    );
}

#[test]
fn respond_not_found() {
    let mut mgr = TicketManager::new();
    assert_eq!(
        mgr.respond(999, "GM_Alice", "Hello", 1000),
        Err(TicketError::NotFound)
    );
}

#[test]
fn drain_clears_notifications() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();

    let first = mgr.drain_notifications();
    assert_eq!(first.len(), 1);

    let second = mgr.drain_notifications();
    assert!(second.is_empty());
}

#[test]
fn full_lifecycle_notifications() {
    let mut mgr = TicketManager::new();
    let id = mgr
        .create(100, TicketCategory::Stuck, "Stuck", 1000)
        .unwrap();

    mgr.assign(id, "GM_Alice").unwrap();
    mgr.respond(id, "GM_Alice", "Unsticking you now", 2000)
        .unwrap();
    mgr.resolve(id).unwrap();

    let notifs = mgr.drain_notifications();
    assert_eq!(notifs.len(), 3);
    assert!(matches!(notifs[0], TicketNotification::Assigned { .. }));
    assert!(matches!(notifs[1], TicketNotification::GmResponse { .. }));
    assert!(matches!(notifs[2], TicketNotification::Resolved { .. }));
}

// --- Ticket history ---

#[test]
fn player_history_all_tickets() {
    let mut mgr = TicketManager::new();
    let id1 = mgr
        .create(100, TicketCategory::Bug, "First bug", 1000)
        .unwrap();
    mgr.assign(id1, "GM_Alice").unwrap();
    mgr.resolve(id1).unwrap();

    let id2 = mgr
        .create(100, TicketCategory::Quest, "Quest issue", 2000)
        .unwrap();
    mgr.assign(id2, "GM_Bob").unwrap();
    mgr.resolve(id2).unwrap();

    let id3 = mgr
        .create(100, TicketCategory::Stuck, "Stuck again", 3000)
        .unwrap();

    let history = mgr.player_history(100);
    assert_eq!(history.len(), 3);
    // Newest first
    assert_eq!(history[0].id, id3);
    assert_eq!(history[1].id, id2);
    assert_eq!(history[2].id, id1);
}

#[test]
fn player_history_empty() {
    let mgr = TicketManager::new();
    assert!(mgr.player_history(100).is_empty());
}

#[test]
fn player_history_only_own_tickets() {
    let mut mgr = TicketManager::new();
    mgr.create(100, TicketCategory::Bug, "Player 100", 1000)
        .unwrap();
    mgr.create(200, TicketCategory::Bug, "Player 200", 1001)
        .unwrap();

    let history = mgr.player_history(100);
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].player, 100);
}

#[test]
fn player_resolved_history() {
    let mut mgr = TicketManager::new();
    let id1 = mgr
        .create(100, TicketCategory::Bug, "Resolved bug", 1000)
        .unwrap();
    mgr.assign(id1, "GM").unwrap();
    mgr.resolve(id1).unwrap();

    // Open ticket (not in resolved history)
    mgr.create(100, TicketCategory::Quest, "Open quest", 2000)
        .unwrap();

    let resolved = mgr.player_resolved_history(100);
    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0].id, id1);
}

#[test]
fn player_ticket_count() {
    let mut mgr = TicketManager::new();
    let id1 = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id1, "GM").unwrap();
    mgr.resolve(id1).unwrap();
    mgr.create(100, TicketCategory::Quest, "Quest", 2000)
        .unwrap();

    assert_eq!(mgr.player_ticket_count(100), 2);
    assert_eq!(mgr.player_ticket_count(200), 0);
}

#[test]
fn history_includes_responses() {
    let mut mgr = TicketManager::new();
    let id = mgr.create(100, TicketCategory::Bug, "Bug", 1000).unwrap();
    mgr.assign(id, "GM_Alice").unwrap();
    mgr.respond(id, "GM_Alice", "Fixed!", 2000).unwrap();
    mgr.resolve(id).unwrap();

    let history = mgr.player_history(100);
    assert_eq!(history[0].responses.len(), 1);
    assert_eq!(history[0].responses[0].message, "Fixed!");
}
