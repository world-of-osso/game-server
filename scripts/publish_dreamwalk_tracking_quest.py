"""Produce SQL for native Dreamwalk's tracking quest; never mutate the source database."""
import argparse
import sqlite3
from pathlib import Path

from retail_content import QUEST_COLUMNS, QUEST_MONEY_DIFFICULTIES

DREAMWALK = 193753
TRACKING_QUEST = 39174
TRACKING_EVENT_FLAG = 0x400


def render_sql(conn):
    native = conn.execute("""SELECT q.RewardBonusMoney FROM tdb_quest_template q
        WHERE q.ID=? AND (q.Flags & ?)<>0
        AND EXISTS(SELECT 1 FROM quest_v2 v WHERE v.ID=q.ID)
        AND EXISTS(SELECT 1 FROM spell_effect e
            WHERE e.SpellID=? AND e.Effect=16 AND e.EffectMiscValue_0=q.ID)""",
        (TRACKING_QUEST, TRACKING_EVENT_FLAG, DREAMWALK)).fetchone()
    if native is None:
        raise ValueError("native Dreamwalk193753→tracking quest39174 definition is unavailable")
    if native[0] != 0:
        raise ValueError("quest39174 now needs a nonzero quest-reward-bonus catalog publication")
    return render_insert()


def render_insert():
    targets = ", ".join(QUEST_COLUMNS)
    sources = ", ".join(f"q.{column}" for column in QUEST_COLUMNS.values())
    difficulty = "CASE q.RewardMoneyDifficulty " + " ".join(
        f"WHEN {n} THEN m.Difficulty_{n}" for n in range(QUEST_MONEY_DIFFICULTIES)) + " ELSE 0 END"
    # Same native columns, scaling level and money calculation as retail_content.insert_quest_templates.
    # No AzerothCore reward substitution: this source tracking row authors zero bonus money.
    return f"""-- Native193753 effect16→39174; retail_content.QUEST_COLUMNS mapping.
BEGIN IMMEDIATE;
INSERT INTO content_quest_template ({targets}, QuestLevel, MinLevel, RewardMoney)
SELECT {sources}, -1, COALESCE(l.minlevel, 0),
    COALESCE(CAST(({difficulty}) * q.RewardMoneyMultiplier AS INTEGER), 0)
FROM tdb_quest_template q
LEFT JOIN content_tuning_level l ON l.ID=q.ContentTuningID
LEFT JOIN quest_money_reward m ON m.ID=l.maxlevel
WHERE q.ID={TRACKING_QUEST}
    AND NOT EXISTS(SELECT 1 FROM content_quest_template existing WHERE existing.ID=q.ID);
COMMIT;
SELECT ID, LogTitle, Flags, ContentTuningID, RewardBonusMoney
FROM content_quest_template WHERE ID={TRACKING_QUEST};
"""


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.output.resolve() == args.source.resolve():
        parser.error("SQL output must not overwrite the source database")
    with sqlite3.connect(f"{args.source.resolve().as_uri()}?mode=ro", uri=True) as conn:
        sql = render_sql(conn)
    args.output.write_text(sql)


if __name__ == "__main__":
    main()
