#!/usr/bin/env node
/**
 * gev_snapshot.mjs — capture a timestamped "world state" snapshot from a
 * running God's Eye View data plane (and whitelisted upstream feeds) into
 * `world-snapshots/` for Jarvis's offline world-sense reader.
 *
 * This is a STAGING tool (like the wheelhouse/model download step): it may
 * use the network while the owner runs it. Jarvis's runtime only reads the
 * resulting files; it never calls this tool itself.
 *
 *   node tools\gev_snapshot.mjs [--api-base http://localhost:4173]
 *                                 [--out world-snapshots]
 *                                 [--feeds flights,launches,...]
 *                                 [--timeout-ms 20000]
 *
 * Output contract (version 1):
 *   <out>/manifest.json      — {version, generatedAtMs, feeds:{key:Entry}}
 *   <out>/feeds/<key>.<ext>  — raw feed body (JSON pretty-printed when valid)
 *
 * Entry: {request, via, ok, fetchedAtMs, httpStatus, contentType, bytes,
 *         sha256, feedSource, feedCache, feedAgeMs, error}
 * `feedCache`/`feedSource`/`feedAgeMs` preserve GEV's own staleness headers
 * (x-feed-cache: STALE, x-tle-cache: STALE-ERROR, x-feed-age-ms, x-*-source)
 * so the reader can never mistake a stale replay for a live answer.
 *
 * A failed feed is recorded with ok:false and error — it does not abort the
 * run. Exit 0 whenever the manifest was written; exit 2 on usage errors.
 */

import { createHash } from 'node:crypto';
import { mkdir, readFile, writeFile, rename } from 'node:fs/promises';
import { join, resolve } from 'node:path';

const DEFAULT_API_BASE = 'http://localhost:4173';
const DEFAULT_OUT = 'world-snapshots';

/**
 * The staged feed set. `via: 'gev'` paths are fetched from the running
 * God's Eye View server; `via: 'upstream'` URLs are fetched directly.
 * `text: true` bodies are stored verbatim (TLE/CSV), not re-serialized.
 * Keep this list aligned with scripts/world_sense.py FEED_* tables.
 */
const FEEDS = Object.freeze([
  { key: 'flights', via: 'gev', path: '/api/flights' },
  { key: 'military', via: 'gev', path: '/api/military' },
  { key: 'vessels', via: 'gev', path: '/api/vessels?maxRows=8000' },
  { key: 'launches', via: 'gev', path: '/api/launches' },
  { key: 'firms', via: 'gev', path: '/api/firms' },
  { key: 'fire_perimeters', via: 'gev', path: '/api/fire-perimeters' },
  { key: 'cyclones', via: 'gev', path: '/api/cyclones' },
  { key: 'news_hotspots', via: 'gev', path: '/api/news/hotspots' },
  { key: 'news_feed', via: 'gev', path: '/api/news/feed?limit=100&geo=1' },
  { key: 'celestrak_stations', via: 'gev', path: '/api/celestrak/stations', text: true },
  { key: 'celestrak_visual', via: 'gev', path: '/api/celestrak/visual', text: true },
  { key: 'satcat_stations', via: 'gev', path: '/api/celestrak-satcat/stations', text: true },
  {
    key: 'earthquakes',
    via: 'upstream',
    url: 'https://earthquake.usgs.gov/earthquakes/feed/v1.0/summary/all_day.geojson',
  },
]);

/** Provenance headers worth preserving per feed. */
const HEADER_FIELDS = Object.freeze([
  ['feedSource', ['x-feed-source', 'x-flight-source']],
  ['feedCache', ['x-feed-cache', 'x-tle-cache', 'x-satcat-cache']],
  ['feedAgeMs', ['x-feed-age-ms']],
  ['coverage', ['x-flight-coverage']],
]);

function parseArgs(argv) {
  const options = { apiBase: DEFAULT_API_BASE, out: DEFAULT_OUT, timeoutMs: 20000 };
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i];
    const next = () => {
      const value = argv[++i];
      if (value === undefined) throw new Error(`Missing value for ${arg}`);
      return value;
    };
    if (arg === '--api-base') options.apiBase = next();
    else if (arg === '--out') options.out = next();
    else if (arg === '--feeds') options.feeds = new Set(next().split(',').map((s) => s.trim()));
    else if (arg === '--timeout-ms') options.timeoutMs = Number(next());
    else throw new Error(`Unknown argument: ${arg}`);
  }
  if (!Number.isFinite(options.timeoutMs) || options.timeoutMs <= 0)
    throw new Error('--timeout-ms must be a positive number');
  return options;
}

function feedUrl(feed, apiBase) {
  if (feed.via === 'upstream') return feed.url;
  const base = new URL(apiBase);
  if (!['http:', 'https:'].includes(base.protocol))
    throw new Error(`api-base must be an http(s) URL: ${apiBase}`);
  return new URL(feed.path, base).toString();
}

async function captureFeed(feed, apiBase, timeoutMs) {
  const request = feedUrl(feed, apiBase);
  const entry = {
    request: feed.via === 'gev' ? feed.path : feed.url,
    via: feed.via,
    ok: false,
    fetchedAtMs: Date.now(),
    httpStatus: null,
    contentType: null,
    bytes: 0,
    sha256: null,
    error: null,
  };
  try {
    const response = await fetch(request, {
      signal: AbortSignal.timeout(timeoutMs),
      headers: { accept: feed.text ? 'text/plain,*/*' : 'application/json,*/*' },
    });
    entry.httpStatus = response.status;
    for (const [field, names] of HEADER_FIELDS) {
      for (const name of names) {
        const value = response.headers.get(name);
        if (value) {
          entry[field] = value;
          break;
        }
      }
    }
    const buffer = Buffer.from(await response.arrayBuffer());
    entry.bytes = buffer.length;
    entry.sha256 = createHash('sha256').update(buffer).digest('hex');
    entry.contentType = (response.headers.get('content-type') || '').split(';')[0] || null;
    if (!response.ok) {
      entry.error = `HTTP ${response.status}`;
      return { entry };
    }
    entry.ok = true;
    return { entry, body: buffer };
  } catch (error) {
    entry.error = error?.name === 'TimeoutError' ? 'timeout' : String(error?.message || error);
    return { entry };
  }
}

async function writeBody(outDir, feed, body) {
  if (body === undefined) return null;
  const feedsDir = join(outDir, 'feeds');
  await mkdir(feedsDir, { recursive: true });
  if (feed.text) {
    const file = `feeds/${feed.key}.txt`;
    await writeFile(join(outDir, file), body);
    return file;
  }
  // Pretty-print valid JSON so the snapshot stays diff/inspection friendly.
  const file = `feeds/${feed.key}.json`;
  try {
    const parsed = JSON.parse(body.toString('utf8'));
    await writeFile(join(outDir, file), JSON.stringify(parsed, null, 2));
  } catch {
    await writeFile(join(outDir, file), body);
  }
  return file;
}

async function writeManifest(outDir, manifest) {
  await mkdir(outDir, { recursive: true });
  const tmp = join(outDir, 'manifest.json.tmp');
  const final = join(outDir, 'manifest.json');
  await writeFile(tmp, JSON.stringify(manifest, null, 2));
  await rename(tmp, final);
}

/** Merge into an existing manifest so --feeds refreshes don't drop others. */
async function readExistingFeeds(outDir) {
  try {
    const prior = JSON.parse(await readFile(join(outDir, 'manifest.json'), 'utf8'));
    if (prior?.version === 1 && prior.feeds && typeof prior.feeds === 'object')
      return prior.feeds;
  } catch {
    // Missing or unreadable manifest: start clean, files remain harmless.
  }
  return {};
}

async function main() {
  const options = parseArgs(process.argv.slice(2));
  const outDir = resolve(options.out);
  const selected = FEEDS.filter((feed) => !options.feeds || options.feeds.has(feed.key));
  const manifest = {
    version: 1,
    generatedAtMs: Date.now(),
    apiBase: options.apiBase,
    feeds: await readExistingFeeds(outDir),
  };
  for (const feed of selected) {
    const { entry, body } = await captureFeed(feed, options.apiBase, options.timeoutMs);
    entry.file = await writeBody(outDir, feed, body);
    manifest.feeds[feed.key] = entry;
    const state = entry.ok ? 'OK' : 'FAIL';
    const detail = entry.ok
      ? `HTTP ${entry.httpStatus} ${entry.bytes}B`
      : entry.error || `HTTP ${entry.httpStatus}`;
    console.log(`${state} ${feed.key}: ${detail}`);
  }
  await writeManifest(outDir, manifest);
  const okCount = Object.values(manifest.feeds).filter((f) => f.ok).length;
  console.log(`Snapshot: ${okCount}/${selected.length} feeds captured -> ${outDir}`);
}

main().catch((error) => {
  console.error(`gev_snapshot: ${error.message}`);
  process.exitCode = 2;
});
