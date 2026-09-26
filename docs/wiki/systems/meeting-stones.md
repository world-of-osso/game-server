# Meeting stones

`crates/server/src/meeting_stone/`. Spec: [meeting stones](../../specs/meeting-stones.md).

- **Data:** `load_meeting_stone_data` reads each type-23 stone's ContentTuning max level (`tdb_gameobject_template.ContentTuningId` → `content_tuning_level`; the merged `content_gameobject_template` lacks the column) and every type-18 ritual template (`Data0` casters, `Data1` spell, `Data6` casters grouped, `GameObjectDisplayInfo.GeoBox`).
- **Use:** `npc_interaction::usable_object` maps types 23 and 18 to `ObjectUse::Spell`; `meeting_stone::use_object` runs TrinityCore's `GameObject::Use` checks (GameObject.cpp:3188-3217) and casts through `spell_cast::cast_granted`. TrinityCore returns silently on a failed check; here the user gets `CastFailed`.
- **Portal:** `SPELL_EFFECT_TRANS_DOOR` (50) is a once-per-cast handler (`effects::cast_handler`). `ritual::effect_trans_door` spawns the MiscValue template on the caster with a `Ritual { owner, channel_spell, casters, spell, grouped, users, expires_at }`. `ritual::channel_ended` (from `spell_cast::cancel_cast`) and `expire_rituals` despawn it.
- **Completion:** `ritual::use_ritual` adds distinct grouped users while the owner channels; at `casters` users it cancels the channel and `cast_triggered`s the ritual spell at the owner's target.
- **Offer:** `summon::effect_summon_player` (85) inserts `PendingSummon { location, expires_at }` on the target and sends `SummonRequest`. `receive_summon_responses` applies `HandleSummonResponseOpcode`: dead or in-combat answers are dropped; an accept in time calls `taxi::finish_flight` and `teleport::teleported`.
- **Tests:** `meeting_stone/tests.rs` drives the real world.db stones 178834 / 178832, portal 179944 and spells 59782 / 7720 through `UseGameObject` and `SummonResponse` messages.
