# Hostile primary across native self casts — 2026-10-24

## Goal and scope

Parent authorized a generic correction on branch `ret-hostile-primary-4996`, based on master `4996e5967d80fa82b92d6590a59bd005f50d0965`. Keep the actual selected action primary separate from the native cast unit when forwarding children. No Storm/Arbiter spell-ID branch, multiplier, target fallback, canonical write or unrelated Ret development import. The separate Ret literal-driver branch remains at33075cb34.

## Genuine RED

Test-only46606eedb uses real2062 four-piece gear, actual native Purpose consumption to arm Verdict kind, naturally earned generator power, canonical Prepared/Ret bindings/shared execution, then an ordinary self-cast Storm. It reaches the opposite-kind consumption boundary and observes the successful native1306923 launch. Actual child target is caster71v0; expected hostile72v0. No pending-kind, resource, eligibility or delayed-flight setup failure precedes the assertion.

`dps_parity_ret_tier_literal_self_cast_preserves_arbiter_hostile_primary`: **genuine RED50.92s**, warning-free2m08s compile. Exact argv/revision/log: engine `data/diagnostics/ret-parity-2026-10-23/hostile-primary-4996/red-v2.{proof.json,log}`. The initial62c01071c attempt had only test-module visibility/type compilation errors; v1 is explicitly not RED. Test moved into its binding module; the production tree remained unchanged before v2.

Pinned SimC76f8118 `sc_paladin.hpp1687–1718` forwards execute-state target; native `ret_tier.rs` forwarded the explicit cast unit. Native1306923 direct/splash AP10.125/4.7249999 match generated SimC data. No coefficient shim is justified.

## Correction and native GREEN

The shared execution record now carries the native cast unit and selected primary separately. Default selection uses the same ordered native candidate as other actions; explicit selection keys retain their own candidate, and an explicit self context carries no hostile primary. Player and NPC cast entry points accept these separate inputs. Native cast/queue/bar snapshots retain the provided primary; foreground child forwarding uses that snapshot rather than the cast unit. Normal wire/script requests keep their own explicit request context. No later combat-target or nearby-unit lookup substitutes for missing context.

At ec5f0f3f3 the unchanged hostile-primary regression and three existing tier controls are **4/4GREEN60.88s**, warning-free2m16s compile. The formerly self-targeted child now receives the actual hostile primary and deals real damage. Two-piece, same/cross-kind and no-set controls retain their prior behavior. Receipt `hostile-primary-4996/green-v1.{proof.json,log}`. This qualifies the actual NPC boundary, not every queued/Player path or full DPS parity.

Parent-required master3f897bd38 (D3/cooldown_query) merged cleanly as81ec6e642; primary implementation is unchanged. New shared controls at81ec give13PASS86.31s: all9 native Player dispatch/knowledge/selected/far/expired/resource/retarget controls plus4row gates. Seven composite tests initially stopped because259/71 input files were absent. After copying only the unchanged pinned inputs, all7PASS72.75s at1595c675e, warning-free2m48s compile (`shared-composite-v2.*`); the13 passing cases were not redundantly repeated. One old movement.distance named-gap assertion fails; no fixture/oracle change or empirical baseline claim was made for it.

The old Zenith target_if control stops before dispatch with `NATIVE_FIXTURE_TARGET_UNAVAILABLE: explicit lifetime/role missing`,45.16s. Parent requested one exact plain-master baseline. At3f897bd3860b871e6947d58594c100cc4726931b, same argv/input/prepared3d2d and this worktree’s own target reproduce the identical assertion in47.05s, warning-free4m10s compile. Classified **pre-existing** by that exact counterfactual; no WW fixture change belongs in this branch. Receipt `zenith-master3f-once.{proof.json,log}`; not rerun.

Core Player queue controls ran once at81ec: **11/11PASS40.79s**, warning-free11m50s compile/helper752.25s (`player-queue-v1.*`). Pyrun detached the long compile as job01a1706e; it completed normally and was not restarted. Existing queue timing, actual builder funding, range/death/interrupt revalidation, replacement/cancellation and override expiry remain qualified.

Test-only91603c369 adds actual Player self-cast coverage using the identical native child-target assertion body. NPC plus Player are **2/2PASS45.11s**, warning-free19.71s compile (`actors-v1.*`). No production change follows81ec. These two overlap the original NPC regression: unique passing coverage is5Ret actor/tier cases plus13Player/gating,7composite and11queue =36cases, not38.

Parent-requested combined `-p server -p server-simulation -p server-spell-tests --tests -j2` check ran **ONCE/PASS119.35s** at91603c369, zero warnings (`three-crate-check-once.*`). `cargo fmt -p server -p server-simulation -- --check`: **PASS10.09s** (`fmt.*`). Plain production `-p server --bins -j2` check has not started: capacity reached SHED at298GiB free, below300GiB. No target/cache deletion or check repetition was made.

The shared/Player source proof does not turn the old movement assertion or Zenith fixture into GREEN. Parent owns integration/final gates. No ready/full parity claim; after the pending plain-bin gate and integration, one fresh canonical4442 backup pair is required, then Dawnlight is next.
