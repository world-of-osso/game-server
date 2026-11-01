#!/usr/bin/env python3
"""Append the current US Mythic+ week from public WCL +7–9 rankings."""
import argparse
import datetime as dt
import json
import os
from pathlib import Path
import subprocess
import tempfile
from zoneinfo import ZoneInfo

PACIFIC = ZoneInfo("America/Los_Angeles")
SEED = Path(__file__).resolve().parents[1] / "crates/server-combat/src/mythic_plus_weeks.json"
POLICY = Path("data/mythic_plus_weeks.json")
BARGAINS = {148, 158, 160, 162}
DEFENSES = {9, 10}
ENCOUNTER = 12993  # WCL Altar of Fangs, Midnight Season 2.


def week_bounds(now):
    local = now.astimezone(PACIFIC)
    days_since_tuesday = (local.weekday() - 1) % 7
    reset_day = local.date() - dt.timedelta(days=days_since_tuesday)
    start = dt.datetime.combine(reset_day, dt.time(8), PACIFIC)
    if local < start:
        reset_day -= dt.timedelta(days=7)
        start = dt.datetime.combine(reset_day, dt.time(8), PACIFIC)
    end = dt.datetime.combine(reset_day + dt.timedelta(days=7), dt.time(8), PACIFIC)
    return int(start.timestamp()), int(end.timestamp())


def read_rankings(bracket, page):
    result = subprocess.run(
        [
            "wcl", "rankings", str(ENCOUNTER), "--difficulty", "10",
            "--bracket", str(bracket), "--page", str(page), "--json",
        ],
        capture_output=True,
        text=True,
    )
    if result.returncode:
        raise RuntimeError(
            f"WCL rankings bracket {bracket} page {page} failed: {result.stderr.strip()}"
        )
    return json.loads(result.stdout)["characterRankings"]


def collect_observations(start, end, now, pages):
    observations = {}
    for bracket in (6, 7):
        for page in range(1, pages + 1):
            rankings = read_rankings(bracket, page)
            for row in rankings["rankings"]:
                add_observation(observations, row, start, end, now)
            if not rankings["hasMorePages"]:
                break
    report_codes = {code for code, _ in observations}
    if len(report_codes) < 2:
        raise ValueError("Need two distinct public US +7–9 reports from this reset week")
    values = set(observations.values())
    if len(values) != 1:
        raise ValueError(f"Conflicting WCL affixes in this reset week: {sorted(values)}")
    return next(iter(values)), sorted(observations)


def add_observation(observations, row, start, end, now):
    if row.get("server", {}).get("region") != "US":
        return
    if not 7 <= row.get("hardModeLevel", 0) <= 9:
        return
    fight_seconds = row.get("startTime", 0) / 1000
    if not start <= fight_seconds < min(end, now):
        return
    affixes = set(row.get("affixes", []))
    bargains, defenses = affixes & BARGAINS, affixes & DEFENSES
    if len(bargains) != 1 or len(defenses) != 1:
        return
    report = row.get("report", {})
    code, fight = report.get("code"), report.get("fightID")
    if not isinstance(code, str) or not code.isalnum():
        return
    if not isinstance(fight, int) or fight < 1:
        return
    report_key = (code, fight)
    values = (next(iter(bargains)), next(iter(defenses)))
    if report_key in observations and observations[report_key] != values:
        raise ValueError(f"Conflicting affixes for WCL report {code} fight {fight}")
    observations[report_key] = values


def append_week(path, start, end, values, reports):
    created = not path.exists()
    table = json.loads((SEED if created else path).read_text())
    bargain, defense = values
    found = False
    for week in table["weeks"]:
        if week["start_unix"] == start:
            if (week["end_unix"], week["bargain"], week["defense"]) != (end, bargain, defense):
                raise ValueError("Observed affixes conflict with existing week; do not overwrite it")
            found = True
            break
        if start < week["end_unix"] and end > week["start_unix"]:
            raise ValueError("Observed week overlaps an existing week")
    if found and not created:
        return False
    if not found:
        sources = "; ".join(
            f"https://www.warcraftlogs.com/reports/{code}#fight={fight}"
            for code, fight in reports
        )
        table["weeks"].append({
            "start_unix": start,
            "end_unix": end,
            "bargain": bargain,
            "defense": defense,
            "citation": sources,
            "defense_citation": sources,
            "qualification": "Observed on distinct public US +7–9 WCL reports.",
        })
        table["weeks"].sort(key=lambda week: week["start_unix"])
    with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, delete=False) as output:
        temporary = Path(output.name)
        source = SEED if created else path
        os.fchmod(output.fileno(), source.stat().st_mode & 0o777)
        json.dump(table, output, indent=2)
        output.write("\n")
    try:
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)
    return True


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--policy", type=Path, default=POLICY)
    parser.add_argument("--as-of", help="ISO 8601 timestamp with timezone; defaults to now")
    parser.add_argument("--pages", type=int, default=10, choices=range(1, 11))
    args = parser.parse_args()
    now = dt.datetime.fromisoformat(args.as_of) if args.as_of else dt.datetime.now(dt.timezone.utc)
    if now.tzinfo is None:
        parser.error("--as-of requires a timezone")
    start, end = week_bounds(now)
    values, reports = collect_observations(start, end, now.timestamp(), args.pages)
    changed = append_week(args.policy, start, end, values, reports)
    action = "Wrote" if changed else "Confirmed"
    print(
        f"{action} US week {start}–{end}: bargain {values[0]}, "
        f"defense {values[1]} from {len({code for code, _ in reports})} reports"
    )


if __name__ == "__main__":
    main()
