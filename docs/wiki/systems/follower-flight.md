# Follower flight

Followers use the leader's flight state, not the leader's appearance, to follow in three dimensions.

## Flight and landing

`DAWNBREAKER_FLIGHT_TRAVERSAL` starts when the leader is flying in an area whose AreaTable `MountFlags` contains `0x2`. The follower leaves its ground route and follows the leader without a navmesh route. A druid uses its own Flight Form. Other followers use the documented default Golden Gryphon because follower profiles do not yet contain mount identities.

When the leader stops flying, each follower moves toward the landing point and requests its navmesh projection. The follower leaves flight only after reaching that surface. Ground AI and normal follow then resume. The simulator leader's flight traversal is a separate pending implementation.

## Sources

- [Follower dungeon contract](../../specs/follower-dungeons.md#flying-follow) — policy and profile-mount gap.
- [Follower flight implementation](../../../crates/server-combat/src/creature_ai/flight_follow.rs) — runtime movement and landing.
- [Follower flight test](../../../crates/server-combat/src/followers/flight_follow_tests.rs) — area gating, follower identities and landing.

## See Also

- [[navigation]] — baked navigation and deferred queries.
- [[dungeon-simulator]] — follower-dungeon simulation.
