"""Unit tests for scripts/world_sense.py — staged world snapshot reader."""

import json
import os
import sys
import tempfile
import unittest

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(REPO_ROOT, "scripts"))
sys.path.insert(0, os.path.join(REPO_ROOT, "tools"))

import world_sense

NOW_MS = 1_800_000_000_000  # fixed reference time for deterministic ages


def write_snapshot(feeds: dict, directory: str) -> None:
    """feeds: {key: {"body": obj|str|None, "age_ms": int, "ok": bool,
    "feedCache": str|None, "error": str|None}}"""
    manifest = {"version": 1, "generatedAtMs": NOW_MS, "feeds": {}}
    feeds_dir = os.path.join(directory, "feeds")
    os.makedirs(feeds_dir, exist_ok=True)
    for key, spec in feeds.items():
        entry = {
            "request": f"/api/{key}",
            "via": "gev",
            "ok": spec.get("ok", True),
            "fetchedAtMs": NOW_MS - spec.get("age_ms", 0),
            "httpStatus": 200 if spec.get("ok", True) else 503,
            "bytes": 10,
            "sha256": "0" * 64,
            "error": spec.get("error"),
        }
        if spec.get("feedCache"):
            entry["feedCache"] = spec["feedCache"]
        body = spec.get("body")
        if body is not None:
            if isinstance(body, str):
                rel = f"feeds/{key}.txt"
                with open(os.path.join(directory, rel), "w", encoding="utf-8") as h:
                    h.write(body)
            else:
                rel = f"feeds/{key}.json"
                with open(os.path.join(directory, rel), "w", encoding="utf-8") as h:
                    json.dump(body, h)
            entry["file"] = rel
        manifest["feeds"][key] = entry
    with open(os.path.join(directory, "manifest.json"), "w", encoding="utf-8") as h:
        json.dump(manifest, h)


QUAKES = {
    "type": "FeatureCollection",
    "features": [
        {
            "properties": {"mag": 5.2, "place": "10km S of Testville", "time": NOW_MS - 3_600_000},
            "geometry": {"coordinates": [122.0, 14.0, 33.0]},
        },
        {
            "properties": {"mag": 4.1, "place": "Offshore Nowhere", "time": NOW_MS - 7_200_000},
            "geometry": {"coordinates": [121.0, 13.0, 10.0]},
        },
    ],
}

LAUNCHES = {
    "results": [
        {"name": "Falcon 9 | Starlink Group 1", "net": "2027-01-05T10:00:00Z",
         "status": {"name": "Go for Launch"}, "pad": {"name": "Cape Canaveral"}},
        {"name": "Electron | Test", "net": "2027-01-09T01:00:00Z",
         "status": {"name": "TBD"}, "pad": {"name": "Mahia"}},
    ]
}

FLIGHTS = {
    "states": [
        ["abc123", "TEST123", "Nowhereland", NOW_MS, NOW_MS, 121.5, 14.2, 11000, False, 250, 90],
        ["def456", "DEMO99", "Elsewhere", NOW_MS, NOW_MS, 100.0, 30.0, 10500, False, 240, 270],
    ]
}

MILITARY = {"ac": [{"hex": "ae1234", "flight": "RCH123", "lat": 14.1, "lon": 121.9, "t": "C17"}]}

CYCLONES = {
    "source": "NOAA NHC / CPHC",
    "storms": [
        {
            "id": "cp042026",
            "name": "Pena",
            "classification": "TS",
            "basin": "CP",
            "position": {"longitude": -167.7, "latitude": 9.9},
            "windKt": 40,
        }
    ],
}

PERIMETERS = {
    "rows": [
        {"stableId": "x1", "name": "RX Buck Creek", "acres": 805,
         "state": "US-OK", "county": "McCurtain", "category": "RX"},
        {"stableId": "x2", "name": "Big Fire", "acres": 4200,
         "state": "US-CA", "county": "Fresno", "category": "WF"},
    ]
}

HOTSPOTS = {
    "hotspots": [
        {"id": "worldwide", "name": "Worldwide", "kind": "conflict",
         "counts": {"last24h": 119}},
        {"id": "ua", "name": "Ukraine", "kind": "conflict",
         "counts": {"last24h": 40}},
    ]
}

TLE = (
    "ISS (ZARYA)\n"
    "1 25544U 98067A   24001.00000000  .00016717  00000-0  10270-3 0  9000\n"
    "2 25544  51.6400 208.9163 0006317  69.9862  25.2906 15.49560532    10\n"
    "CSS (TIANHE)\n"
    "1 48274U 21035A   24001.00000000  .00020000  00000-0  20000-3 0  9001\n"
    "2 48274  41.4700 100.0000 0007000  50.0000  20.0000 15.60000000    20\n"
)

NEWS = {
    "reports": [
        {"title": "Test headline one", "publisher": "wire"},
        {"title": "Second headline", "publisher": "wire"},
    ]
}


class ManifestTests(unittest.TestCase):
    def test_missing_manifest_returns_none(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertIsNone(world_sense._load_manifest(d))

    def test_load_manifest(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot({"earthquakes": {"body": QUAKES}}, d)
            manifest = world_sense._load_manifest(d)
            self.assertIsNotNone(manifest)
            self.assertIn("earthquakes", manifest["feeds"])

    def test_wrong_version_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            with open(os.path.join(d, "manifest.json"), "w") as h:
                json.dump({"version": 99, "feeds": {}}, h)
            self.assertIsNone(world_sense._load_manifest(d))


class FeedStateTests(unittest.TestCase):
    def test_nominal_within_window(self):
        entry = {"ok": True, "fetchedAtMs": NOW_MS - 60_000, "_key": "flights"}
        self.assertEqual(world_sense.feed_state(entry, NOW_MS), "nominal")

    def test_stale_past_window(self):
        entry = {"ok": True, "fetchedAtMs": NOW_MS - 3_600_000, "_key": "flights"}
        self.assertEqual(world_sense.feed_state(entry, NOW_MS), "stale")

    def test_event_feed_stays_fresh_longer(self):
        entry = {"ok": True, "fetchedAtMs": NOW_MS - 3_600_000, "_key": "earthquakes"}
        self.assertEqual(world_sense.feed_state(entry, NOW_MS), "nominal")

    def test_missing_and_unavailable(self):
        self.assertEqual(world_sense.feed_state(None, NOW_MS), "missing")
        self.assertEqual(
            world_sense.feed_state({"ok": False, "_key": "vessels"}, NOW_MS),
            "unavailable",
        )

    def test_server_stale_cache_is_degraded(self):
        entry = {
            "ok": True,
            "fetchedAtMs": NOW_MS - 1000,
            "feedCache": "STALE-ERROR",
            "_key": "celestrak_stations",
        }
        self.assertEqual(world_sense.feed_state(entry, NOW_MS), "degraded")


class ParserTests(unittest.TestCase):
    def test_earthquakes(self):
        rows = world_sense.parse_earthquakes(QUAKES)
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0]["mag"], 5.2)
        self.assertEqual(rows[0]["lat"], 14.0)

    def test_tle(self):
        rows = world_sense.parse_tle(TLE)
        self.assertEqual(len(rows), 2)
        self.assertIn("ISS", rows[0]["name"])

    def test_opensky_states(self):
        rows = world_sense.parse_flights(FLIGHTS)
        self.assertEqual(len(rows), 2)
        self.assertEqual(rows[0]["name"], "TEST123")

    def test_readsb_ac(self):
        rows = world_sense.parse_military(MILITARY)
        self.assertEqual(rows[0]["name"], "RCH123")

    def test_bad_shape_raises(self):
        with self.assertRaises(ValueError):
            world_sense.parse_earthquakes({"not": "geojson"})
        with self.assertRaises(ValueError):
            world_sense.parse_tle("not a tle")
        with self.assertRaises(ValueError):
            world_sense.parse_flights({"states": []})


class AnswerTests(unittest.TestCase):
    def test_non_world_text_returns_none(self):
        with tempfile.TemporaryDirectory() as d:
            self.assertIsNone(
                world_sense.answer_world_query("tell me a joke", NOW_MS, d)
            )

    def test_no_manifest_honest_reply(self):
        with tempfile.TemporaryDirectory() as d:
            out = world_sense.answer_world_query("any earthquakes today?", NOW_MS, d)
            self.assertIsNotNone(out)
            self.assertFalse(out.ok)
            self.assertEqual(out.reason, "no_snapshot")
            self.assertIn("No world snapshot", out.reply)

    def test_earthquakes_answered_with_counts(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot({"earthquakes": {"body": QUAKES}}, d)
            out = world_sense.answer_world_query("any earthquakes today?", NOW_MS, d)
            self.assertTrue(out.ok)
            self.assertEqual(out.feed_state, "nominal")
            self.assertIn("2 earthquakes", out.reply)
            self.assertIn("M5.2", out.reply)
            self.assertIn("Testville", out.reply)

    def test_stale_feed_is_labeled(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot(
                {"flights": {"body": FLIGHTS, "age_ms": 3_600_000}}, d
            )
            out = world_sense.answer_world_query(
                "what aircraft are flying?", NOW_MS, d
            )
            self.assertTrue(out.ok)
            self.assertEqual(out.feed_state, "stale")
            self.assertIn("STALE", out.reply)

    def test_failed_feed_reports_error(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot(
                {"vessels": {"ok": False, "error": "HTTP 503"}}, d
            )
            out = world_sense.answer_world_query("any vessels nearby?", NOW_MS, d)
            self.assertFalse(out.ok)
            self.assertEqual(out.reason, "feed_unavailable")
            self.assertIn("503", out.reply)

    def test_missing_feed_reports_not_staged(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot({"news_feed": {"body": NEWS}}, d)
            out = world_sense.answer_world_query("show me ship positions", NOW_MS, d)
            self.assertFalse(out.ok)
            self.assertIn("no ships data staged", out.reply)

    def test_parse_failure_is_honest(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot(
                {"earthquakes": {"body": {"unexpected": "shape"}}}, d
            )
            out = world_sense.answer_world_query("recent quakes?", NOW_MS, d)
            self.assertFalse(out.ok)
            self.assertEqual(out.reason, "parse_failed")

    def test_launches_and_news(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot(
                {"launches": {"body": LAUNCHES}, "news_feed": {"body": NEWS}}, d
            )
            out = world_sense.answer_world_query("upcoming rocket launches?", NOW_MS, d)
            self.assertTrue(out.ok)
            self.assertIn("Falcon 9", out.reply)
            out = world_sense.answer_world_query("latest news headlines", NOW_MS, d)
            self.assertIn("Test headline one", out.reply)

    def test_cyclones_real_shape(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot({"cyclones": {"body": CYCLONES}}, d)
            out = world_sense.answer_world_query("any hurricanes?", NOW_MS, d)
            self.assertTrue(out.ok)
            self.assertIn("Pena", out.reply)
            self.assertIn("40", out.reply)

    def test_fire_falls_back_to_perimeters(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot(
                {
                    "firms": {"ok": False, "error": "missing-key"},
                    "fire_perimeters": {"body": PERIMETERS},
                },
                d,
            )
            out = world_sense.answer_world_query("any wildfires?", NOW_MS, d)
            self.assertTrue(out.ok)
            self.assertEqual(out.feed_key, "fire_perimeters")
            self.assertIn("2 tracked fire perimeters", out.reply)
            self.assertIn("Big Fire", out.reply)

    def test_hotspots_answer(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot({"news_hotspots": {"body": HOTSPOTS}}, d)
            out = world_sense.answer_world_query(
                "what conflicts are happening?", NOW_MS, d
            )
            self.assertTrue(out.ok)
            self.assertIn("159", out.reply)
            self.assertIn("Ukraine", out.reply)

    def test_satellite_answer_is_honest_about_positions(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot({"celestrak_stations": {"body": TLE}}, d)
            out = world_sense.answer_world_query(
                "what satellites are overhead?", NOW_MS, d
            )
            self.assertTrue(out.ok)
            self.assertIn("catalog records, not live coordinates", out.reply)

    def test_status_reply(self):
        with tempfile.TemporaryDirectory() as d:
            write_snapshot(
                {
                    "earthquakes": {"body": QUAKES},
                    "vessels": {"ok": False, "error": "missing-key"},
                },
                d,
            )
            out = world_sense.answer_world_query("world snapshot status", NOW_MS, d)
            self.assertTrue(out.ok)
            self.assertIn("earthquakes", out.reply)
            self.assertIn("unavailable", out.reply)


class QueryRegexTests(unittest.TestCase):
    def test_matches_world_domains(self):
        for text in (
            "any earthquakes today",
            "what satellites are overhead",
            "military aircraft nearby",
            "latest news headlines",
            "any hurricanes forming",
            "upcoming rocket launches",
            "what aircraft are flying overhead",
            "any wildfires burning",
            "vessels in the area",
            "what conflicts are happening",
            "god's eye view status",
        ):
            self.assertRegex(text, world_sense.WORLD_QUERY_RE, text)

    def test_does_not_swallow_other_intents(self):
        for text in (
            "what is the weather today",
            "check my flight gate",
            "what time is it",
            "cpu usage",
            "what can you do",
        ):
            self.assertIsNone(world_sense.WORLD_QUERY_RE.search(text), text)


if __name__ == "__main__":
    unittest.main()
