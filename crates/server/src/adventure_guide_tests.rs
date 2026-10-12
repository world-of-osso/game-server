use super::*;
use lightyear::prelude::{MessageReceiver, MessageSender};
use shared::protocol::{AdventureGuideRequest, AdventureGuideSnapshot, InstanceChannel};

#[test]
fn adventure_guide_query_replies_instead_of_inventing_missing_catalog_content() {
    let file = tempfile::NamedTempFile::new().unwrap();
    let mut h = crate::bank_test_harness::harness_at(file.path(), "Journalrealm").unwrap();
    AdventureGuidePlugin.build(&mut h.app);
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<AdventureGuideRequest>::default(),
        MessageReceiver::<AdventureGuideRequest>::default(),
        MessageSender::<AdventureGuideSnapshot>::default(),
        MessageReceiver::<AdventureGuideSnapshot>::default(),
    ));
    h.app
        .world_mut()
        .get_mut::<MessageSender<AdventureGuideRequest>>(h.connection)
        .unwrap()
        .send::<InstanceChannel>(AdventureGuideRequest::Query);
    let mut replies = Vec::new();
    for _ in 0..10 {
        h.app.update();
        replies.extend(
            h.app
                .world_mut()
                .get_mut::<MessageReceiver<AdventureGuideSnapshot>>(h.connection)
                .unwrap()
                .receive(),
        );
    }
    assert_eq!(
        replies.len(),
        1,
        "realm-backed journal request must actually receive a response"
    );
    assert!(replies[0].source_error.is_some());
    assert!(replies[0].activities.is_empty());
    assert!(replies[0].suggestions.is_empty());
}
