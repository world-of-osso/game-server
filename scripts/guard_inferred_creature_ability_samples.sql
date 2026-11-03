-- Apply to the existing world database after invalid rows are corrected.
-- Stop and roll back on any error. Never delete source samples here.
BEGIN IMMEDIATE;
CREATE TEMP TABLE _inferred_sample_guard(ok INTEGER NOT NULL CHECK(ok = 1));
INSERT INTO _inferred_sample_guard SELECT CASE WHEN NOT EXISTS (
    SELECT 1 FROM content_inferred_creature_ability_sample
    WHERE timing_kind NOT IN ('first_activity_proxy', 'successive_cast')
       OR typeof(delay_ms) != 'integer' OR delay_ms < 0 OR delay_ms > 4294967295
       OR (timing_kind = 'successive_cast' AND delay_ms = 0)
) THEN 1 ELSE 0 END;
CREATE TRIGGER IF NOT EXISTS inferred_sample_validate_insert
BEFORE INSERT ON content_inferred_creature_ability_sample
WHEN NEW.timing_kind NOT IN ('first_activity_proxy', 'successive_cast')
  OR typeof(NEW.delay_ms) != 'integer' OR NEW.delay_ms < 0 OR NEW.delay_ms > 4294967295
  OR (NEW.timing_kind = 'successive_cast' AND NEW.delay_ms = 0)
BEGIN
    SELECT RAISE(ABORT, 'invalid inferred ability sample timing');
END;
CREATE TRIGGER IF NOT EXISTS inferred_sample_validate_update
BEFORE UPDATE ON content_inferred_creature_ability_sample
WHEN NEW.timing_kind NOT IN ('first_activity_proxy', 'successive_cast')
  OR typeof(NEW.delay_ms) != 'integer' OR NEW.delay_ms < 0 OR NEW.delay_ms > 4294967295
  OR (NEW.timing_kind = 'successive_cast' AND NEW.delay_ms = 0)
BEGIN
    SELECT RAISE(ABORT, 'invalid inferred ability sample timing');
END;
DROP TABLE _inferred_sample_guard;
COMMIT;
