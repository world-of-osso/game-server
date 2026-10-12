# Adventure Guide realm service

The shared [Adventure Guide contract](../../../game-engine/docs/specs/adventure-guide.md) owns client/realm behavior and the approved monthly policy. The [system page](../../../game-engine/docs/wiki/systems/adventure-guide.md#realm-activities-and-suggestions) owns implementation and proof boundaries.

Catalog publication uses `scripts/adventure_guide_catalog.py` against the pinned local source and an explicit read-only world reference. It emits SQL/report artifacts; applying them to shared realm data is separate deployment work. Missing catalog data produces an explicit unavailable snapshot, not fabricated activities or progress.
