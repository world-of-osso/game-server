use super::*;

#[test]
fn purchase_charter_success() {
    let mut mgr = PetitionManager::new();
    let id = mgr.purchase_charter(100, "Test Guild", 5000, false, 1000).unwrap();

    assert_eq!(id, 1);
    let petition = mgr.get(id).unwrap();
    assert_eq!(petition.owner, 100);
    assert_eq!(petition.guild_name, "Test Guild");
    assert!(petition.signatures.is_empty());
    assert_eq!(petition.created_at, 1000);
}

#[test]
fn purchase_costs_10_silver() {
    let mut mgr = PetitionManager::new();
    // Exactly 10 silver (1000 copper) — should succeed
    assert!(mgr.purchase_charter(100, "Guild", CHARTER_COST, false, 1000).is_ok());
}

#[test]
fn purchase_insufficient_funds() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        mgr.purchase_charter(100, "Guild", CHARTER_COST - 1, false, 1000),
        Err(PetitionError::InsufficientFunds)
    );
}

#[test]
fn purchase_already_guilded() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        mgr.purchase_charter(100, "Guild", 5000, true, 1000),
        Err(PetitionError::AlreadyInGuild)
    );
}

#[test]
fn purchase_already_has_petition() {
    let mut mgr = PetitionManager::new();
    mgr.purchase_charter(100, "First Guild", 5000, false, 1000).unwrap();
    assert_eq!(
        mgr.purchase_charter(100, "Second Guild", 5000, false, 1001),
        Err(PetitionError::AlreadyHasPetition)
    );
}

#[test]
fn purchase_empty_name() {
    let mut mgr = PetitionManager::new();
    assert_eq!(
        mgr.purchase_charter(100, "", 5000, false, 1000),
        Err(PetitionError::InvalidName)
    );
}

#[test]
fn purchase_name_too_long() {
    let mut mgr = PetitionManager::new();
    let long = "x".repeat(MAX_GUILD_NAME_LEN + 1);
    assert_eq!(
        mgr.purchase_charter(100, &long, 5000, false, 1000),
        Err(PetitionError::InvalidName)
    );
}

#[test]
fn purchase_name_max_length_ok() {
    let mut mgr = PetitionManager::new();
    let name = "x".repeat(MAX_GUILD_NAME_LEN);
    assert!(mgr.purchase_charter(100, &name, 5000, false, 1000).is_ok());
}

#[test]
fn purchase_name_taken_by_existing_guild() {
    let mut mgr = PetitionManager::new();
    mgr.reserve_name("Stormwind Guard");
    assert_eq!(
        mgr.purchase_charter(100, "stormwind guard", 5000, false, 1000),
        Err(PetitionError::NameTaken)
    );
}

#[test]
fn purchase_name_taken_by_pending_petition() {
    let mut mgr = PetitionManager::new();
    mgr.purchase_charter(100, "Cool Guild", 5000, false, 1000).unwrap();
    assert_eq!(
        mgr.purchase_charter(200, "cool guild", 5000, false, 1001),
        Err(PetitionError::NameTaken)
    );
}

#[test]
fn find_by_owner() {
    let mut mgr = PetitionManager::new();
    mgr.purchase_charter(100, "Guild", 5000, false, 1000).unwrap();
    assert!(mgr.find_by_owner(100).is_some());
    assert!(mgr.find_by_owner(200).is_none());
}

#[test]
fn cancel_petition() {
    let mut mgr = PetitionManager::new();
    let id = mgr.purchase_charter(100, "Guild", 5000, false, 1000).unwrap();
    mgr.cancel(id, 100).unwrap();
    assert!(mgr.is_empty());
}

#[test]
fn cancel_not_owner_fails() {
    let mut mgr = PetitionManager::new();
    let id = mgr.purchase_charter(100, "Guild", 5000, false, 1000).unwrap();
    assert_eq!(mgr.cancel(id, 200), Err(PetitionError::NotOwner));
}

#[test]
fn cancel_not_found() {
    let mut mgr = PetitionManager::new();
    assert_eq!(mgr.cancel(999, 100), Err(PetitionError::NotFound));
}

#[test]
fn sequential_ids() {
    let mut mgr = PetitionManager::new();
    let id1 = mgr.purchase_charter(100, "Guild A", 5000, false, 1000).unwrap();
    mgr.cancel(id1, 100).unwrap();
    let id2 = mgr.purchase_charter(200, "Guild B", 5000, false, 1001).unwrap();
    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}
