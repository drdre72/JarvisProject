"""Read-only inspection of the jarvis-core operational journal.

Stage A evidence per ADR-0006: for every trace, which events the
supervisor saw and what it decided -- plus a shadow-invariant check that
no consequential dispatch ever reached the journal.

Usage:
    python tools/inspect_rust_journal.py [db_path] [--last N] [--check-shadow]
"""

import sqlite3
import sys
from pathlib import Path

# Modes written while shadow_mode is active: "shadow" dispatches and
# "attention" verdict records. "live" would mean Stage B/C escalation.
SHADOW_MODES = {"shadow", "attention"}

DEFAULT_DB = Path("logs/rust-core/operational-journal.db")


def summarize(db_path):
    """Group journaled events by trace_id and collect dispatch records."""
    conn = sqlite3.connect(str(db_path))
    try:
        events = conn.execute(
            "SELECT trace_id, event_id, source, event_type, class,"
            "       disposition, reason, occurred_at_utc"
            " FROM events ORDER BY received_at_utc"
        ).fetchall()
        dispatches = conn.execute(
            "SELECT attempt_id, event_id, mode, outcome, dispatched_at_utc"
            " FROM dispatch_attempts ORDER BY dispatched_at_utc"
        ).fetchall()
    finally:
        conn.close()
    traces = {}
    for row in events:
        traces.setdefault(row[0], []).append(row)
    return {"traces": traces, "dispatches": dispatches}


def shadow_violations(db_path):
    """Any dispatch outside shadow bookkeeping is a Stage A violation."""
    summary = summarize(db_path)
    return [d for d in summary["dispatches"] if d[2] not in SHADOW_MODES]


def main(argv):
    db_path = Path(argv[1]) if len(argv) > 1 and not argv[1].startswith("-") else DEFAULT_DB
    last = int(argv[argv.index("--last") + 1]) if "--last" in argv else 20
    check = "--check-shadow" in argv

    if not db_path.is_file():
        print(f"no journal at {db_path}")
        return 2

    summary = summarize(db_path)
    print(f"journal: {db_path}")
    print(f"events: {sum(len(v) for v in summary['traces'].values())} "
          f"across {len(summary['traces'])} traces; "
          f"dispatches: {len(summary['dispatches'])}\n")
    shown = 0
    for trace_id, rows in summary["traces"].items():
        if shown >= last:
            break
        print(f"trace {trace_id}")
        for _, event_id, source, etype, cls, disp, reason, occurred in rows:
            detail = f" ({reason})" if reason else ""
            print(f"  {occurred} {etype:<26} {source:<14} "
                  f"{cls:<22} {disp}{detail}")
        for attempt_id, event_id, mode, outcome, _ in summary["dispatches"]:
            if any(event_id == row[1] for row in rows):
                print(f"    -> {mode} dispatch {attempt_id}: {outcome}")
        shown += 1

    violations = shadow_violations(db_path)
    if check or violations:
        if violations:
            print("\nSHADOW VIOLATIONS:")
            for v in violations:
                print(f"  {v}")
            return 1
        print("\nshadow invariant holds: no consequential dispatch recorded")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
