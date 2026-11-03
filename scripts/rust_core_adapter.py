"""Python side of the jarvis-core JSON-lines frame contract (ADR-0006).

Mirrors jarvis-core/src/transport.rs and events.rs exactly: frames are
versioned, size-capped in UTF-8 bytes *before* parsing, and event envelopes
are validated before they may occupy queue space. Stage A speaks JSON-lines
over stdio only -- sockets are never used (ADR-0001).

Payload discipline: the Rust journal persists every admitted envelope's
payload_json. Payloads therefore carry operational metadata only -- reason
codes, metrics, evidence references -- never transcripts, prompts, or raw
audio. Content payloads belong to the Markdown vault, not this channel.
"""

import itertools
import json
import os
import queue
import signal
import subprocess
import sys
import threading
import time
from collections import deque
from datetime import datetime, timedelta, timezone
from pathlib import Path

PROTOCOL_VERSION = 1
SCHEMA_VERSION = 1
MAX_FRAME_BYTES = 64 * 1024
MAX_ID_LEN = 64
MAX_DEDUP_KEY_LEN = 128
MAX_FUTURE_SKEW_SECONDS = 300
DEFAULT_MAX_PAYLOAD_BYTES = 16 * 1024

# serde rename_all="snake_case" on EventSource; variant names on EventType.
SOURCES = frozenset(
    {
        "wake_word",
        "ui",
        "voice_pipeline",
        "scheduler",
        "worker_supervisor",
        "resource_monitor",
        "external",
        "internal",
    }
)
EVENT_TYPES = frozenset(
    {
        "WakeWordDetected",
        "UserRequestReceived",
        "ScheduledObligationDue",
        "WorkerHeartbeat",
        "WorkerExited",
        "ResourceSample",
        "MaintenanceWindow",
        "ExternalStimulus",
        "ShutdownRequested",
    }
)
FRAME_KINDS = frozenset({"event", "request", "response", "heartbeat", "shutdown"})

_id_counter = itertools.count(1)
_id_lock = threading.Lock()


class FrameError(ValueError):
    """Typed mirror of transport.rs TransportError / events.rs EventError."""

    def __init__(self, reason, detail=""):
        self.reason = reason
        super().__init__(f"{reason}: {detail}" if detail else reason)


def new_id(prefix):
    """Rust ids::new_id format: '{prefix}-{nanos:016x}-{counter:06x}'."""
    with _id_lock:
        n = next(_id_counter)
    return f"{prefix}-{time.time_ns():016x}-{n:06x}"


def rfc3339_now():
    return _rfc3339(datetime.now(timezone.utc))


def _rfc3339(dt):
    return dt.astimezone(timezone.utc).isoformat().replace("+00:00", "Z")


def _parse_rfc3339(text):
    if not isinstance(text, str):
        raise FrameError("malformed", "timestamp is not a string")
    try:
        return datetime.fromisoformat(text.replace("Z", "+00:00"))
    except ValueError:
        raise FrameError("malformed", f"bad rfc3339 {text!r}")


def _json_bytes(value):
    return json.dumps(value, separators=(",", ":"), ensure_ascii=False).encode("utf-8")


def validate_envelope(envelope, max_payload_bytes=DEFAULT_MAX_PAYLOAD_BYTES):
    """Strict ingress validation, mirroring EventEnvelope::validate."""
    if envelope.get("schema_version") != SCHEMA_VERSION:
        raise FrameError(
            "unsupported_version", f"schema_version={envelope.get('schema_version')}"
        )
    for field in ("event_id", "trace_id"):
        value = envelope.get(field)
        if not isinstance(value, str) or not 1 <= len(value) <= MAX_ID_LEN:
            raise FrameError("bad_id", field)
    dedup_key = envelope.get("dedup_key")
    if dedup_key is not None and len(dedup_key) > MAX_DEDUP_KEY_LEN:
        raise FrameError("oversized_dedup_key")
    if envelope.get("source") not in SOURCES:
        raise FrameError("malformed", f"unknown source {envelope.get('source')!r}")
    if envelope.get("event_type") not in EVENT_TYPES:
        raise FrameError("malformed", f"unknown event_type {envelope.get('event_type')!r}")
    try:
        payload_size = len(_json_bytes(envelope.get("payload")))
    except (TypeError, ValueError) as error:
        raise FrameError("malformed", f"payload not serializable: {error}")
    if payload_size > max_payload_bytes:
        raise FrameError("oversized_payload", str(max_payload_bytes))
    occurred = _parse_rfc3339(envelope.get("occurred_at_utc"))
    if occurred > datetime.now(timezone.utc) + timedelta(seconds=MAX_FUTURE_SKEW_SECONDS):
        raise FrameError("future_timestamp")


def event_envelope(source, event_type, payload=None, dedup_key=None, occurred_at=None):
    """Supervisor-side event constructor, mirroring EventEnvelope::create."""
    envelope = {
        "schema_version": SCHEMA_VERSION,
        "event_id": new_id("evt"),
        "trace_id": new_id("trace"),
        "source": source,
        "event_type": event_type,
        "occurred_at_utc": _rfc3339(occurred_at)
        if occurred_at is not None
        else rfc3339_now(),
        "dedup_key": dedup_key,
        "payload": payload if payload is not None else {},
    }
    validate_envelope(envelope)
    return envelope


def frame(body):
    return {
        "protocol_version": PROTOCOL_VERSION,
        "msg_id": new_id("msg"),
        "sent_at_utc": rfc3339_now(),
        "body": body,
    }


def event_frame(source, event_type, payload=None, dedup_key=None, occurred_at=None):
    return frame(
        {
            "kind": "event",
            "data": event_envelope(
                source, event_type, payload, dedup_key, occurred_at
            ),
        }
    )


def heartbeat_frame(worker):
    return frame({"kind": "heartbeat", "data": {"worker": worker}})


def request_frame(op, args=None, request_id=None):
    return frame(
        {
            "kind": "request",
            "data": {
                "request_id": request_id or new_id("req"),
                "op": op,
                "args": args if args is not None else {},
            },
        }
    )


def shutdown_frame(reason=None):
    return frame({"kind": "shutdown", "data": {"reason": reason}})


def obligation(
    obligation_id,
    next_run_at_utc,
    scope,
    origin="python-worker",
    priority=5,
    recurrence_seconds=None,
    expires_at_utc=None,
    missed_policy="run_once_on_recovery",
):
    """Build a scheduler Obligation dict (jarvis-core scheduler.rs shape).

    Datetimes may be datetime objects or RFC3339 strings. Status is always
    "pending" here; the supervisor re-normalizes untrusted fields anyway.
    """
    def stamp(value):
        return _rfc3339(value) if isinstance(value, datetime) else value

    return {
        "obligation_id": obligation_id,
        "created_at_utc": rfc3339_now(),
        "next_run_at_utc": stamp(next_run_at_utc),
        "origin": origin,
        "scope": scope,
        "priority": int(priority),
        "recurrence_seconds": recurrence_seconds,
        "expires_at_utc": stamp(expires_at_utc) if expires_at_utc is not None else None,
        "missed_policy": missed_policy,
        "last_result": None,
        "status": "pending",
    }


def encode(wire_frame):
    """Serialize one frame as a single JSON line, capped like Rust encode()."""
    raw = _json_bytes(wire_frame)
    if len(raw) > MAX_FRAME_BYTES:
        raise FrameError("oversized", str(MAX_FRAME_BYTES))
    return raw.decode("utf-8") + "\n"


def decode(line, max_bytes=MAX_FRAME_BYTES):
    """Parse one line; mirrors transport::decode (envelope validated separately)."""
    trimmed = line.rstrip()
    if not trimmed:
        raise FrameError("empty")
    if len(trimmed.encode("utf-8")) > max_bytes:
        raise FrameError("oversized", str(max_bytes))
    try:
        wire = json.loads(trimmed)
    except json.JSONDecodeError as error:
        raise FrameError("malformed", str(error))
    if not isinstance(wire, dict):
        raise FrameError("malformed", "frame is not an object")
    if wire.get("protocol_version") != PROTOCOL_VERSION:
        raise FrameError("unsupported_version", wire.get("protocol_version"))
    msg_id = wire.get("msg_id")
    if not isinstance(msg_id, str) or not 1 <= len(msg_id) <= MAX_ID_LEN:
        raise FrameError("bad_id")
    _parse_rfc3339(wire.get("sent_at_utc"))
    body = wire.get("body")
    if not isinstance(body, dict) or body.get("kind") not in FRAME_KINDS:
        raise FrameError("malformed", "unknown or missing body kind")
    return wire


def write_config(
    path,
    data_dir,
    enabled,
    shadow_mode=True,
    event_queue_capacity=256,
    dedup_window_seconds=30,
    max_payload_bytes=DEFAULT_MAX_PAYLOAD_BYTES,
    per_source_burst=32,
    ipc_requests=False,
    scheduler_tick_seconds=1,
    max_due_per_tick=8,
    heartbeat_timeout_seconds=30,
    max_restart_attempts=3,
    base_backoff_ms=500,
    max_backoff_ms=30000,
):
    """Write a jarvis-core.toml. Mirrors jarvis-core/jarvis-core.toml.example."""
    data_dir_toml = str(data_dir).replace("\\", "/")
    text = f"""[rust_core]
enabled = {str(enabled).lower()}
shadow_mode = {str(shadow_mode).lower()}
data_dir = "{data_dir_toml}"
event_queue_capacity = {event_queue_capacity}
dedup_window_seconds = {dedup_window_seconds}
max_payload_bytes = {max_payload_bytes}
per_source_burst = {per_source_burst}
ipc_requests = {str(ipc_requests).lower()}

[scheduler]
tick_seconds = {scheduler_tick_seconds}
max_due_per_tick = {max_due_per_tick}

[workers]
heartbeat_timeout_seconds = {heartbeat_timeout_seconds}
max_restart_attempts = {max_restart_attempts}
base_backoff_ms = {base_backoff_ms}
max_backoff_ms = {max_backoff_ms}
"""
    Path(path).write_text(text, encoding="utf-8")
    return Path(path)


def find_binary(repo_root):
    """Locate the debug-built jarvis-core binary, if one exists."""
    name = "jarvis-core.exe" if os.name == "nt" else "jarvis-core"
    candidate = Path(repo_root) / "jarvis-core" / "target" / "debug" / name
    return candidate if candidate.is_file() else None


class RustCoreProcess:
    """Owns one jarvis-core child process on stdio JSON-lines.

    stdout lines that decode as valid frames land in ``frames``; everything
    else (tracing logs in Stage A) is kept in ``log_lines`` for diagnostics.
    stderr is captured separately so a panic is never mistaken for a frame.
    """

    def __init__(self, binary_path, config_path, data_dir=None, env=None):
        self.binary_path = str(binary_path)
        self.config_path = str(config_path)
        self.data_dir = Path(data_dir) if data_dir is not None else None
        self.env = dict(env) if env else {}
        self.frames = queue.Queue()
        self.log_lines = deque(maxlen=600)
        self._proc = None
        self._threads = []

    @property
    def journal_path(self):
        if self.data_dir is None:
            return None
        return self.data_dir / "operational-journal.db"

    def start(self):
        if self._proc is not None:
            raise RuntimeError("jarvis-core already started")
        creationflags = 0
        if os.name == "nt":
            creationflags = subprocess.CREATE_NEW_PROCESS_GROUP
        # EnvFilter::from_default_env() is ERROR-only without RUST_LOG; the
        # intake's warn!/info! diagnostics need at least info to be visible.
        child_env = os.environ.copy()
        child_env.update(self.env)
        child_env.setdefault("RUST_LOG", "info")
        self._proc = subprocess.Popen(
            [self.binary_path, "--config", self.config_path],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            env=child_env,
            creationflags=creationflags,
        )
        for stream, tag in ((self._proc.stdout, "out"), (self._proc.stderr, "err")):
            thread = threading.Thread(
                target=self._pump, args=(stream, tag), daemon=True
            )
            thread.start()
            self._threads.append(thread)
        return self

    def _pump(self, stream, tag):
        try:
            for raw in stream:
                line = raw.decode("utf-8", errors="replace").rstrip("\r\n")
                if not line:
                    continue
                try:
                    self.frames.put(decode(line))
                except FrameError:
                    self.log_lines.append(f"{tag}|{line}")
        except (OSError, ValueError):
            pass

    def is_running(self):
        return self._proc is not None and self._proc.poll() is None

    def send_line(self, line):
        """Write one frame line to stdin (str or bytes, newline optional)."""
        if self._proc is None or self._proc.stdin is None:
            raise RuntimeError("jarvis-core is not started")
        raw = line if isinstance(line, bytes) else line.encode("utf-8")
        if not raw.endswith(b"\n"):
            raw += b"\n"
        try:
            if self._proc.stdin.closed:
                raise RuntimeError("jarvis-core stdin closed")
            self._proc.stdin.write(raw)
            self._proc.stdin.flush()
        except (BrokenPipeError, OSError, ValueError) as error:
            raise RuntimeError(f"jarvis-core stdin closed: {error}")

    def send_wire_frame(self, wire_frame):
        self.send_line(encode(wire_frame))
        return wire_frame["msg_id"]

    def send_event(self, source, event_type, payload=None, dedup_key=None):
        wire = event_frame(source, event_type, payload, dedup_key)
        self.send_wire_frame(wire)
        return wire["body"]["data"]["event_id"]

    def send_raw(self, text):
        """Deliberately unvalidated bytes -- for malformed-input probes."""
        self.send_line(text)

    def request(self, op, args=None, request_id=None):
        wire = request_frame(op, args, request_id)
        self.send_wire_frame(wire)
        return wire["body"]["data"]["request_id"]

    def next_frame(self, timeout=5.0):
        try:
            return self.frames.get(timeout=timeout)
        except queue.Empty:
            raise TimeoutError("no frame from jarvis-core before deadline")

    def await_response(self, request_id, timeout=5.0):
        """Collect frames until the matching response arrives (Stage B)."""
        deadline = time.monotonic() + timeout
        while True:
            remaining = deadline - time.monotonic()
            if remaining <= 0:
                raise TimeoutError(f"no response for {request_id}")
            wire = self.next_frame(timeout=remaining)
            body = wire.get("body", {})
            if body.get("kind") == "response" and body.get("data", {}).get(
                "request_id"
            ) == request_id:
                return body["data"]

    def close_stdin(self):
        if self._proc is not None and self._proc.stdin is not None:
            try:
                self._proc.stdin.close()
            except OSError:
                pass

    def stop(self, timeout=5.0):
        """Protocol shutdown first; Ctrl-C, terminate, then kill as fallback."""
        if self._proc is None:
            return
        stdin_open = self._proc.stdin is not None and not self._proc.stdin.closed
        if self.is_running() and stdin_open:
            try:
                self.send_wire_frame(shutdown_frame("adapter_stop"))
            except RuntimeError:
                pass
            try:
                self._proc.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                pass
        if self.is_running():
            try:
                if os.name == "nt":
                    self._proc.send_signal(signal.CTRL_C_EVENT)
                else:
                    self._proc.send_signal(signal.SIGINT)
            except OSError:
                pass
            try:
                self._proc.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                self._proc.terminate()
                try:
                    self._proc.wait(timeout=2.0)
                except subprocess.TimeoutExpired:
                    self._proc.kill()
                    self._proc.wait(timeout=2.0)
        # Let the pump threads drain the pipes to EOF so log_lines and
        # frames are complete when stop() returns, then close the streams.
        for thread in self._threads:
            thread.join(timeout=2.0)
        for stream in (self._proc.stdin, self._proc.stdout, self._proc.stderr):
            try:
                if stream is not None:
                    stream.close()
            except OSError:
                pass
        self._threads = []
        self._proc = None

    def __enter__(self):
        return self.start()

    def __exit__(self, *_):
        self.stop()


__all__ = [
    "DEFAULT_MAX_PAYLOAD_BYTES",
    "EVENT_TYPES",
    "FRAME_KINDS",
    "FrameError",
    "MAX_DEDUP_KEY_LEN",
    "MAX_FRAME_BYTES",
    "MAX_FUTURE_SKEW_SECONDS",
    "MAX_ID_LEN",
    "PROTOCOL_VERSION",
    "RustCoreProcess",
    "SCHEMA_VERSION",
    "SOURCES",
    "decode",
    "encode",
    "event_envelope",
    "event_frame",
    "find_binary",
    "frame",
    "heartbeat_frame",
    "new_id",
    "obligation",
    "request_frame",
    "rfc3339_now",
    "shutdown_frame",
    "validate_envelope",
    "write_config",
]
