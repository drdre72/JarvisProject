"""Offline world-state reference for Jarvis, read from staged snapshots.

`tools/gev_snapshot.mjs` (an owner-run staging step, like the wheelhouse)
captures public-data feeds from a running God's Eye View server into
`world-snapshots/`. This module only reads those files. It performs no
network access, matching Jarvis's offline-at-runtime rule: the answer is
always labeled with how old the snapshot is and which feed produced it.

Every result is a typed WorldAnswer; stale or missing feeds produce honest
typed reasons, never a fabricated count. Reuses God's Eye View's feed-state
vocabulary (nominal / degraded / stale / unavailable / missing).
"""

from __future__ import annotations

import json
import math
import os
import re
from dataclasses import dataclass
from datetime import datetime, timezone

SNAPSHOT_VERSION = 1

# Decay windows: how long a snapshot can answer before it is labeled stale.
# Event feeds (quakes, launches, storms) stay true longer than position feeds.
FEED_FRESHNESS_SEC = {
    "flights": 300,
    "military": 300,
    "vessels": 900,
    "earthquakes": 6 * 3600,
    "launches": 7 * 86400,
    "firms": 86400,
    "fire_perimeters": 86400,
    "cyclones": 12 * 3600,
    "news_feed": 6 * 3600,
    "news_hotspots": 86400,
    "celestrak_stations": 7 * 86400,
    "celestrak_visual": 7 * 86400,
    "satcat_stations": 7 * 86400,
}

FEED_LABELS = {
    "flights": "live flights",
    "military": "military aircraft",
    "vessels": "ships",
    "earthquakes": "earthquakes",
    "launches": "launches",
    "firms": "active fires",
    "fire_perimeters": "fire perimeters",
    "cyclones": "tropical cyclones",
    "news_feed": "news headlines",
    "news_hotspots": "world event hotspots",
    "celestrak_stations": "space stations",
    "celestrak_visual": "visibly bright satellites",
    "satcat_stations": "the satellite catalog",
}

# World questions this module can answer deterministically. The decision
# controller matches this before its live-data refusal, so snapshot-backed
# domains answer from data and everything else still refuses honestly.
WORLD_QUERY_RE = re.compile(
    r"\b(?:earthquakes?|quakes?|tremors?|seismic|"
    r"launches?|rocket launches?|liftoff|space\s?x|starship|"
    r"iss\b|space station|starlink|satellites?|"
    r"military (?:aircraft|jets?|planes?)|warplanes?|"
    r"flights? overhead|aircraft|airplanes?|planes? overhead|"
    r"what(?:'s| is| are) flying|airspace|"
    r"vessels?|warships?|ships?|marine traffic|maritime|"
    r"wildfires?|bushfires?|active fires?|fire perimeters?|"
    r"hurricanes?|cyclones?|typhoons?|tropical storms?|"
    r"conflicts?|protests?|unrest|hotspots?|"
    r"(?:latest |world |top )?(?:news|headlines)|"
    r"world snapshot|world status|god'?s?[- ]?eye)\b",
    re.I,
)


@dataclass(frozen=True)
class WorldAnswer:
    ok: bool
    reply: str
    feed_key: str | None = None
    feed_state: str | None = None
    age_sec: float | None = None
    source: str | None = None
    reason: str | None = None


def snapshot_dir() -> str:
    override = os.environ.get("JARVIS_WORLD_SNAPSHOTS")
    if override:
        return override
    base = os.environ.get("JARVIS_BASE_PATH") or os.path.dirname(
        os.path.dirname(os.path.abspath(__file__))
    )
    return os.path.join(base, "world-snapshots")


def _load_manifest(directory: str | None = None) -> dict | None:
    path = os.path.join(directory or snapshot_dir(), "manifest.json")
    try:
        with open(path, "r", encoding="utf-8") as handle:
            manifest = json.load(handle)
    except (OSError, ValueError):
        return None
    if manifest.get("version") != SNAPSHOT_VERSION or not isinstance(
        manifest.get("feeds"), dict
    ):
        return None
    return manifest


def _age_sec(entry: dict, now: float) -> float | None:
    fetched = entry.get("fetchedAtMs")
    if not isinstance(fetched, (int, float)) or fetched <= 0:
        return None
    return max(0.0, (now - fetched) / 1000.0)


def age_label(age_sec: float | None) -> str:
    if age_sec is None:
        return "an unknown time ago"
    if age_sec < 60:
        return "under a minute ago"
    if age_sec < 3600:
        return f"{int(age_sec // 60)} minutes ago"
    if age_sec < 86400:
        return f"{int(age_sec // 3600)} hours ago"
    return f"{int(age_sec // 86400)} days ago"


def feed_state(entry: dict | None, now: float) -> str:
    """nominal / stale / degraded / unavailable / missing (GEV vocabulary)."""
    if entry is None:
        return "missing"
    if not entry.get("ok"):
        return "unavailable"
    cache = str(entry.get("feedCache") or "").upper()
    if "STALE" in cache:
        return "degraded"
    age = _age_sec(entry, now)
    if age is None:
        return "degraded"
    if age > FEED_FRESHNESS_SEC.get(entry.get("_key", ""), 6 * 3600):
        return "stale"
    return "nominal"


def _feed_entry(manifest: dict, key: str) -> dict | None:
    entry = manifest["feeds"].get(key)
    if isinstance(entry, dict):
        entry = dict(entry)
        entry["_key"] = key
    return entry


def _read_feed_body(directory: str, entry: dict):
    rel = entry.get("file")
    if not rel or ".." in rel.replace("\\", "/").split("/"):
        raise ValueError("snapshot feed path is invalid")
    path = os.path.join(directory, rel)
    if not os.path.isfile(path):
        raise FileNotFoundError(rel)
    with open(path, "r", encoding="utf-8", errors="replace") as handle:
        if rel.endswith(".txt"):
            return handle.read()
        return json.load(handle)


def _haversine_km(lat1: float, lon1: float, lat2: float, lon2: float) -> float:
    radius = 6371.0
    p1, p2 = math.radians(lat1), math.radians(lat2)
    dp, dl = math.radians(lat2 - lat1), math.radians(lon2 - lon1)
    a = math.sin(dp / 2) ** 2 + math.cos(p1) * math.cos(p2) * math.sin(dl / 2) ** 2
    return 2 * radius * math.asin(min(1.0, math.sqrt(a)))


def _num(value) -> float | None:
    try:
        out = float(value)
    except (TypeError, ValueError):
        return None
    return out if math.isfinite(out) else None


def _pick(row: dict, *names):
    for name in names:
        value = row.get(name)
        if value not in (None, ""):
            return value
    return None


# --- Per-feed parsers: defensive duck-typing, never a guessed schema. ---
# Each returns a list of normalized dicts or raises ValueError.


def _parse_geojson_features(body) -> list[dict]:
    if isinstance(body, dict) and isinstance(body.get("features"), list):
        return body["features"]
    raise ValueError("not a geojson FeatureCollection")


def parse_earthquakes(body) -> list[dict]:
    quakes = []
    for feature in _parse_geojson_features(body):
        props = feature.get("properties") or {}
        coords = ((feature.get("geometry") or {}).get("coordinates")) or []
        quake = {
            "mag": _num(props.get("mag")),
            "place": props.get("place") or "unknown location",
            "time_ms": _num(props.get("time")),
            "lon": _num(coords[0]) if len(coords) > 0 else None,
            "lat": _num(coords[1]) if len(coords) > 1 else None,
            "depth_km": _num(coords[2]) if len(coords) > 2 else None,
        }
        if quake["mag"] is not None:
            quakes.append(quake)
    if not quakes:
        raise ValueError("no earthquake features")
    return quakes


def parse_launches(body) -> list[dict]:
    rows = None
    if isinstance(body, dict):
        for key in ("results", "launches", "data", "items"):
            if isinstance(body.get(key), list):
                rows = body[key]
                break
    elif isinstance(body, list):
        rows = body
    if rows is None:
        raise ValueError("unrecognized launches shape")
    launches = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        pad = row.get("pad") if isinstance(row.get("pad"), dict) else {}
        status = row.get("status") if isinstance(row.get("status"), dict) else {}
        launches.append(
            {
                "name": _pick(row, "name", "title", "mission") or "unnamed launch",
                "when": _pick(row, "net", "window_start", "time", "date"),
                "status": _pick(status, "name", "abbrev"),
                "where": _pick(pad, "name", "location", "country"),
            }
        )
    if not launches:
        raise ValueError("no launch records")
    return launches


def _first_list(body, keys: tuple[str, ...]) -> list | None:
    if isinstance(body, list):
        return body
    if isinstance(body, dict):
        for key in keys:
            if isinstance(body.get(key), list):
                return body[key]
    return None


def _position_rows(body, keys: tuple[str, ...]) -> list[dict]:
    """Normalize rows of position reports: OpenSky 'states' arrays or dict lists."""
    rows = _first_list(body, keys)
    if rows is None:
        raise ValueError("unrecognized position shape")
    out = []
    for row in rows:
        if isinstance(row, dict):
            lat = _num(_pick(row, "lat", "latitude"))
            lon = _num(_pick(row, "lon", "lng", "longitude"))
            name = _pick(row, "callsign", "flight", "name", "hex", "icao24", "mmsi", "id")
            name = (str(name).strip() or None) if name is not None else None
            kind = _pick(row, "t", "type", "category", "aircraft_type", "ship_type")
            alt = _num(_pick(row, "alt_baro", "baro_altitude", "altitude", "alt_geom"))
            out.append({"lat": lat, "lon": lon, "name": name, "kind": kind, "alt_m": alt})
        elif isinstance(row, (list, tuple)) and len(row) >= 7:
            out.append(
                {
                    "lat": _num(row[6]),
                    "lon": _num(row[5]),
                    "name": str(row[1] or row[0] or "").strip() or None,
                    "kind": None,
                    "alt_m": _num(row[7]) if len(row) > 7 else None,
                }
            )
    return [r for r in out if r["lat"] is not None and r["lon"] is not None]


def parse_flights(body) -> list[dict]:
    rows = _position_rows(body, ("states", "aircraft", "flights", "ac", "results", "data"))
    if not rows:
        raise ValueError("no positioned aircraft")
    return rows


def parse_military(body) -> list[dict]:
    rows = _position_rows(body, ("ac", "aircraft", "states", "results", "data"))
    if not rows:
        raise ValueError("no positioned military aircraft")
    return rows


def parse_vessels(body) -> list[dict]:
    rows = _position_rows(body, ("rows", "vessels", "ships", "results", "data", "features"))
    if not rows:
        raise ValueError("no positioned vessels")
    return rows


def parse_fires(body) -> list[dict]:
    if isinstance(body, dict) and isinstance(body.get("features"), list):
        rows = []
        for feature in body["features"]:
            props = feature.get("properties") or {}
            coords = ((feature.get("geometry") or {}).get("coordinates")) or []
            rows.append(
                {
                    "lat": _num(coords[1]) if len(coords) > 1 else None,
                    "lon": _num(coords[0]) if len(coords) > 0 else None,
                    "name": _pick(props, "name", "IncidentName", "attr_IncidentName"),
                    "kind": _pick(props, "satellite", "source"),
                }
            )
        rows = [r for r in rows if r["lat"] is not None]
        if rows:
            return rows
    rows = _position_rows(body, ("fires", "results", "data", "items"))
    if not rows:
        raise ValueError("no fire records")
    return rows


def parse_cyclones(body) -> list[dict]:
    # GEV /api/cyclones: {storms: [{name, classification, basin,
    #   position: {latitude, longitude}, windKt, ...}]}
    rows = _first_list(body, ("storms", "activeStorms", "features", "results", "data"))
    if rows is None:
        raise ValueError("unrecognized cyclone shape")
    storms = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        pos = row.get("position") if isinstance(row.get("position"), dict) else {}
        name = _pick(row, "name", "stormName", "id")
        storms.append(
            {
                "name": name or "unnamed storm",
                "kind": _pick(row, "classification", "stormType", "type", "basin"),
                "wind_kt": _num(row.get("windKt")),
                "lat": _num(pos.get("latitude") or row.get("latitude") or row.get("lat")),
                "lon": _num(pos.get("longitude") or row.get("longitude") or row.get("lon")),
            }
        )
    if not storms:
        raise ValueError("no cyclone records")
    return storms


def parse_fire_perimeters(body) -> list[dict]:
    # GEV /api/fire-perimeters: {rows: [{name, acres, containedPct, state,
    #   county, category, polygons}]}
    rows = _first_list(body, ("rows", "fires", "results", "data"))
    if rows is None:
        raise ValueError("unrecognized perimeter shape")
    fires = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        fires.append(
            {
                "name": row.get("name") or "unnamed fire",
                "acres": _num(row.get("acres")),
                "contained_pct": _num(row.get("containedPct")),
                "state": row.get("state"),
                "county": row.get("county"),
                "category": row.get("category"),
            }
        )
    if not fires:
        raise ValueError("no perimeter records")
    return fires


def parse_news(body) -> list[dict]:
    # GEV /api/news/feed: {reports: [{title, publisher, category, severity,
    #   publishedAt, geo: {place}}]}
    rows = _first_list(body, ("reports", "items", "articles", "news", "results", "data"))
    if rows is None:
        raise ValueError("unrecognized news shape")
    items = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        title = _pick(row, "title", "headline", "name")
        if title:
            items.append(
                {
                    "title": str(title),
                    "source": _pick(row, "publisher", "source", "feed"),
                    "place": (row.get("geo") or {}).get("place")
                    if isinstance(row.get("geo"), dict)
                    else None,
                }
            )
    if not items:
        raise ValueError("no news items")
    return items


def parse_news_hotspots(body) -> list[dict]:
    # GEV /api/news/hotspots: {hotspots: [{name, kind, counts: {last24h}}]}
    rows = _first_list(body, ("hotspots", "items", "results", "data"))
    if rows is None:
        raise ValueError("unrecognized hotspots shape")
    spots = []
    for row in rows:
        if not isinstance(row, dict):
            continue
        counts = row.get("counts") if isinstance(row.get("counts"), dict) else {}
        spots.append(
            {
                "name": row.get("name") or "unnamed hotspot",
                "kind": row.get("kind"),
                "last24h": _num(counts.get("last24h")),
            }
        )
    if not spots:
        raise ValueError("no hotspot records")
    return spots


def parse_tle(text: str) -> list[dict]:
    if not isinstance(text, str):
        raise ValueError("tle body is not text")
    lines = [line.strip() for line in text.splitlines() if line.strip()]
    sats = []
    index = 0
    while index + 2 < len(lines):
        name, l1, l2 = lines[index : index + 3]
        if l1.startswith("1 ") and l2.startswith("2 "):
            sats.append(
                {"name": name.removeprefix("0 ").strip(), "line1": l1, "line2": l2}
            )
            index += 3
        else:
            index += 1
    if not sats:
        raise ValueError("no TLE triples")
    return sats


def _format_time(epoch_ms: float | None) -> str:
    if epoch_ms is None:
        return "unknown time"
    try:
        return datetime.fromtimestamp(epoch_ms / 1000.0, timezone.utc).strftime(
            "%H:%M UTC"
        )
    except (OSError, OverflowError, ValueError):
        return "unknown time"


def _provenance(state: str, age: float | None, source: str | None) -> str:
    origin = f" from {source}" if source else ""
    if state == "nominal":
        return f"World snapshot{origin}, taken {age_label(age)}. "
    if state == "degraded":
        return (
            f"World snapshot{origin} from a degraded feed, "
            f"{age_label(age)} old — last-known data. "
        )
    return (
        f"STALE world snapshot{origin}, {age_label(age)} old — "
        "treat this as last-known, not current. "
    )


def _source_label(entry: dict) -> str | None:
    """Human-speakable origin: provider header, 'God's Eye View' for the
    local data plane, or the upstream hostname for direct captures."""
    named = str(entry.get("feedSource") or "").strip()
    if named:
        return named
    if entry.get("via") == "gev":
        return "God's Eye View"
    request = str(entry.get("request") or "")
    match = re.match(r"https?://([^/?\s]+)", request)
    return match.group(1) if match else None


def _unavailable_reply(key: str, entry: dict | None, state: str) -> str:
    label = FEED_LABELS.get(key, key)
    if state == "missing":
        return (
            f"The world snapshot has no {label} data staged, sir. "
            "That feed was not captured when the snapshot was taken."
        )
    error = (entry or {}).get("error")
    detail = f" ({error})" if error else ""
    return (
        f"The staged {label} feed failed to capture{detail}, sir. "
        "It should refresh the next time the snapshotter runs online."
    )


def _nearest(rows: list[dict], lat: float, lon: float, limit: int = 3) -> list[dict]:
    return sorted(rows, key=lambda r: _haversine_km(lat, lon, r["lat"], r["lon"]))[:limit]


def _home_position() -> tuple[float, float] | None:
    lat, lon = _num(os.environ.get("JARVIS_HOME_LAT")), _num(
        os.environ.get("JARVIS_HOME_LON")
    )
    return (lat, lon) if lat is not None and lon is not None else None


# --- Feed answer formatters: (rows, state, age, source) -> reply text. ---


def _fmt_earthquakes(rows, state, age, source):
    prefix = _provenance(state, age, source)
    biggest = sorted(rows, key=lambda q: -(q["mag"] or 0))[:3]
    parts = [
        f"M{q['mag']:.1f} {q['place']} at {_format_time(q['time_ms'])}" for q in biggest
    ]
    return (
        f"{prefix}{len(rows)} earthquakes in the past day window. "
        f"Largest: {'; '.join(parts)}, sir."
    )


def _fmt_launches(rows, state, age, source):
    prefix = _provenance(state, age, source)
    upcoming = rows[:3]
    parts = []
    for row in upcoming:
        when = str(row["when"] or "time TBD")[:10]
        where = f" from {row['where']}" if row["where"] else ""
        parts.append(f"{row['name']} ({when}{where})")
    return f"{prefix}{len(rows)} launches on the schedule. Next: {'; '.join(parts)}, sir."


def _fmt_positions(label: str):
    def formatter(rows, state, age, source):
        prefix = _provenance(state, age, source)
        home = _home_position()
        if home:
            near = _nearest(rows, home[0], home[1])
            parts = [
                f"{r['name'] or 'unknown'} about "
                f"{int(_haversine_km(home[0], home[1], r['lat'], r['lon']))} km away"
                for r in near
            ]
            return (
                f"{prefix}{len(rows)} {label} in the snapshot. "
                f"Nearest to home: {'; '.join(parts)}, sir."
            )
        named = [r["name"] for r in rows if r["name"]][:3]
        tail = f" including {', '.join(named)}" if named else ""
        return f"{prefix}{len(rows)} {label} in the snapshot{tail}, sir."

    return formatter


def _fmt_fires(rows, state, age, source, key):
    prefix = _provenance(state, age, source)
    if key == "fire_perimeters":
        biggest = sorted(rows, key=lambda r: -(r["acres"] or 0))[:3]
        parts = []
        for r in biggest:
            where = ", ".join(p for p in (r["county"], r["state"]) if p)
            size = f" — {int(r['acres'])} acres" if r["acres"] else ""
            parts.append(f"{r['name']}{size} {('in ' + where) if where else ''}")
        return (
            f"{prefix}{len(rows)} tracked fire perimeters. "
            f"Largest: {'; '.join(parts)}, sir."
        )
    named = [r["name"] for r in rows if r["name"]][:3]
    tail = f" Named: {', '.join(named)}." if named else ""
    return f"{prefix}{len(rows)} active fire records in the snapshot.{tail} sir."


def _fmt_cyclones(rows, state, age, source, key):
    prefix = _provenance(state, age, source)
    parts = []
    for r in rows[:4]:
        detail = r["kind"] or ""
        if r["wind_kt"]:
            detail = f"{detail} {int(r['wind_kt'])} kt".strip()
        parts.append(f"{r['name']} ({detail})" if detail else str(r["name"]))
    return f"{prefix}{len(rows)} tracked tropical systems: {'; '.join(parts)}, sir."


def _fmt_news(rows, state, age, source, key):
    prefix = _provenance(state, age, source)
    parts = [r["title"] for r in rows[:4]]
    return f"{prefix}Top reports: {'; '.join(parts)}, sir."


def _fmt_hotspots(rows, state, age, source, key):
    prefix = _provenance(state, age, source)
    total = sum(int(r["last24h"] or 0) for r in rows)
    named = [
        f"{r['name']} ({int(r['last24h'])} events)"
        for r in rows
        if r["last24h"] and not r["name"].lower() == "worldwide"
    ][:3]
    tail = f" Named hotspots: {'; '.join(named)}." if named else ""
    regions = "region" if len(rows) == 1 else "regions"
    return (
        f"{prefix}{total} tracked events across {len(rows)} hotspot "
        f"{regions} in the last 24 hours.{tail} sir."
    )


def _fmt_satellites(group_label: str):
    def formatter(rows, state, age, source, key):
        prefix = _provenance(state, age, source)
        names = [r["name"] for r in rows if r["name"]]
        iss = [n for n in names if "ISS" in n.upper() or "STATION" in n.upper()]
        tail = f" including {', '.join((iss or names)[:3])}" if names else ""
        return (
            f"{prefix}{len(rows)} {group_label} element sets are staged{tail}, sir. "
            "Positions need orbital propagation, which is not built yet — "
            "these are catalog records, not live coordinates."
        )

    return formatter


def _fmt_positions_wrapper(label: str):
    def formatter(rows, state, age, source, key):
        return _fmt_positions(label)(rows, state, age, source)

    return formatter


def _fmt_earthquakes_w(rows, state, age, source, key):
    return _fmt_earthquakes(rows, state, age, source)


def _fmt_launches_w(rows, state, age, source, key):
    return _fmt_launches(rows, state, age, source)


# Per-key parsers for the fires fallback (differently-shaped feeds).
_FIRE_PARSERS = {"firms": parse_fires, "fire_perimeters": parse_fire_perimeters}

# Ordered (pattern, candidate feed keys, formatter, parser). First pattern
# match wins; the first feed key with captured data answers, so a keyed
# feed (firms) can fall back to a keyless one (fire_perimeters).
_RULES = (
    (re.compile(r"earthquakes?|quakes?|tremors?|seismic", re.I), ("earthquakes",), _fmt_earthquakes_w, parse_earthquakes),
    (re.compile(r"launches?|rocket|liftoff|space\s?x|starship", re.I), ("launches",), _fmt_launches_w, parse_launches),
    (re.compile(r"iss|space station", re.I), ("celestrak_stations", "celestrak_visual"), _fmt_satellites("station"), parse_tle),
    (re.compile(r"starlink|satellites?", re.I), ("celestrak_visual", "celestrak_stations"), _fmt_satellites("satellite"), parse_tle),
    (re.compile(r"military|warplanes?|fighter", re.I), ("military",), _fmt_positions_wrapper("military aircraft"), parse_military),
    (re.compile(r"vessels?|warships?|marine traffic|maritime|ships?|boats?", re.I), ("vessels",), _fmt_positions_wrapper("vessels"), parse_vessels),
    (re.compile(r"wildfires?|bushfires?|active fires?|fire perimeters?", re.I), ("firms", "fire_perimeters"), _fmt_fires, _FIRE_PARSERS),
    (re.compile(r"hurricanes?|cyclones?|typhoons?|tropical storms?", re.I), ("cyclones",), _fmt_cyclones, parse_cyclones),
    (re.compile(r"conflicts?|protests?|hotspots?|happening", re.I), ("news_hotspots",), _fmt_hotspots, parse_news_hotspots),
    (re.compile(r"news|headlines?", re.I), ("news_feed",), _fmt_news, parse_news),
    (re.compile(r"flights?|aircraft|airplanes?|planes?|flying|airspace", re.I), ("flights",), _fmt_positions_wrapper("aircraft"), parse_flights),
)


def snapshot_status(now: float | None = None, directory: str | None = None) -> dict:
    """Per-feed state report, for capability/status answers and tests."""
    now = now if now is not None else datetime.now(timezone.utc).timestamp() * 1000
    manifest = _load_manifest(directory)
    if manifest is None:
        return {"ok": False, "reason": "no_snapshot", "feeds": {}}
    feeds = {}
    for key in FEED_FRESHNESS_SEC:
        entry = _feed_entry(manifest, key)
        feeds[key] = {
            "state": feed_state(entry, now),
            "age_sec": _age_sec(entry, now) if entry else None,
            "source": (entry or {}).get("feedSource"),
            "error": (entry or {}).get("error"),
        }
    return {"ok": True, "generated_at_ms": manifest.get("generatedAtMs"), "feeds": feeds}


def status_reply(now: float | None = None, directory: str | None = None) -> WorldAnswer:
    status = snapshot_status(now, directory)
    if not status["ok"]:
        return WorldAnswer(
            False,
            "No world snapshot is staged yet, sir. "
            "It is built by an explicit online step, not by me.",
            reason="no_snapshot",
        )
    counts = {"nominal": [], "stale": [], "degraded": [], "unavailable": [], "missing": []}
    for key, info in status["feeds"].items():
        counts.setdefault(info["state"], []).append(FEED_LABELS.get(key, key))
    parts = []
    for state in ("nominal", "stale", "degraded", "unavailable"):
        if counts.get(state):
            parts.append(f"{state}: {', '.join(counts[state])}")
    return WorldAnswer(
        True,
        "World snapshot status — " + "; ".join(parts) + ", sir.",
        feed_state="mixed",
    )


def answer_world_query(
    text: str,
    now: float | None = None,
    directory: str | None = None,
) -> WorldAnswer | None:
    """Answer a world-state question from staged snapshots, or None when no
    rule matches (callers can then fall through to conversation)."""
    text = (text or "").strip()
    if not text:
        return None
    now = now if now is not None else datetime.now(timezone.utc).timestamp() * 1000
    if re.search(r"world snapshot|world status|god'?s?[- ]?eye", text, re.I):
        return status_reply(now, directory)
    for pattern, keys, formatter, parser in _RULES:
        if not pattern.search(text):
            continue
        manifest = _load_manifest(directory)
        if manifest is None:
            return WorldAnswer(
                False,
                "No world snapshot is staged yet, sir. "
                "It is built by an explicit online step, not by me.",
                feed_key=keys[0],
                reason="no_snapshot",
            )
        chosen = None
        for key in keys:
            candidate = _feed_entry(manifest, key)
            if feed_state(candidate, now) not in ("missing", "unavailable"):
                chosen = (key, candidate)
                break
        if chosen is None:
            entry = _feed_entry(manifest, keys[0])
            state = feed_state(entry, now)
            return WorldAnswer(
                False,
                _unavailable_reply(keys[0], entry, state),
                feed_key=keys[0],
                feed_state=state,
                reason=f"feed_{state}",
            )
        key, entry = chosen
        state = feed_state(entry, now)
        parse = parser if callable(parser) else parser[key]
        try:
            body = _read_feed_body(directory or snapshot_dir(), entry)
            rows = parse(body)
        except (OSError, ValueError, KeyError, IndexError):
            return WorldAnswer(
                False,
                f"The staged {FEED_LABELS.get(key, key)} data could not be "
                "read just now, sir. I would rather say so than guess.",
                feed_key=key,
                feed_state=state,
                reason="parse_failed",
            )
        source = _source_label(entry)
        age = _age_sec(entry, now)
        return WorldAnswer(
            True,
            formatter(rows, state, age, source, key),
            feed_key=key,
            feed_state=state,
            age_sec=age,
            source=source,
        )
    return None
