"""Optional Stage A shadow feed: emit real turn events to jarvis-core.

Active only when the repo-root ``jarvis-core.toml`` parses with
``[rust_core] enabled = true`` and a built binary exists. Every send is
bounded and failure-isolated -- a dead or slow supervisor never breaks a
turn. Payloads are operational metadata only (input kind, outcome codes,
timings): the journal persists them, so transcripts and prompts never
belong on this channel.
"""

import subprocess
import threading
import tomllib
from datetime import datetime, timedelta, timezone
from pathlib import Path

import rust_core_adapter as adapter
from runtime_log import get_runtime_logger

CONFIG_NAME = "jarvis-core.toml"
PID_FILE = "feed.pid"
SUPERVISOR_IMAGE = "jarvis-core"
_MAX_SEND_FAILURES = 8

# The one function Python delegates to Rust in Stage B: a durable daily
# reminder that review proposals may be pending approval. Stable ID makes
# re-registration at each launch idempotent; the due dispatch stays
# shadow-evaluated like everything else until Stage C.
REVIEW_REMINDER_ID = "obl-review-reminder"
REVIEW_REMINDER_RECURRENCE_S = 24 * 60 * 60


def _repo_root():
    return Path(__file__).resolve().parent.parent


def _supervisor_alive(pid):
    """True when a live process with this PID is a jarvis-core binary."""
    try:
        result = subprocess.run(
            ["tasklist", "/FI", f"PID eq {pid}", "/FO", "CSV", "/NH"],
            capture_output=True,
            text=True,
            timeout=10,
        )
    except (OSError, subprocess.SubprocessError):
        return False
    return SUPERVISOR_IMAGE in result.stdout.lower()


def _reap_stale_supervisor(data_dir):
    """Kill a leftover shadow supervisor from a crashed or killed app run.

    jarvis-core is designed to outlive a dead parent, and the feed spawns
    one instance per app launch — so a stale survivor must be reaped before
    a fresh process binds the same journal directory. The pid file scopes
    this to supervisors the feed itself spawned (a manually run jarvis-core
    is left alone), and the image-name check defends against PID reuse.
    Returns the reaped pid or None.
    """
    try:
        pid = int((data_dir / PID_FILE).read_text(encoding="utf-8").strip())
    except (OSError, ValueError):
        return None
    if not _supervisor_alive(pid):
        return None
    try:
        subprocess.run(
            ["taskkill", "/PID", str(pid), "/F"], capture_output=True, timeout=10
        )
    except (OSError, subprocess.SubprocessError):
        return None
    get_runtime_logger().info("reaped stale shadow supervisor pid=%d", pid)
    return pid


class RustCoreFeed:
    """Failure-isolated event emitter over one RustCoreProcess."""

    def __init__(self, process, pid_path=None):
        self._process = process
        self._pid_path = Path(pid_path) if pid_path is not None else None
        self._lock = threading.Lock()
        self.emit_errors = 0
        self.dead = False

    @property
    def process(self):
        return self._process

    def new_trace(self):
        return adapter.new_id("trace")

    def emit(self, source, event_type, payload=None, dedup_key=None, trace_id=None):
        """Send one event envelope. Returns False when the feed gave up."""
        if self.dead:
            return False
        try:
            with self._lock:
                self._process.send_event(
                    source, event_type, payload, dedup_key, trace_id=trace_id
                )
            return True
        except Exception as error:  # a feed fault must never surface in a turn
            self.emit_errors += 1
            if not self._process.is_running() or self.emit_errors >= _MAX_SEND_FAILURES:
                self.dead = True
                get_runtime_logger().warning(
                    "rust core feed disabled type=%s errors=%d",
                    type(error).__name__,
                    self.emit_errors,
                )
            return False

    def request(self, op, args=None, timeout=2.0):
        """Send one Stage B IPC request; returns the response data or None.

        Failure-isolated like emit: a transport fault never surfaces in a
        turn, it just disables the feed. The lock spans send+await so two
        requests can't interleave on one stdin.
        """
        if self.dead:
            return None
        try:
            with self._lock:
                request_id = self._process.request(op, args)
                return self._process.await_response(request_id, timeout=timeout)
        except Exception as error:
            self.emit_errors += 1
            if not self._process.is_running() or self.emit_errors >= _MAX_SEND_FAILURES:
                self.dead = True
                get_runtime_logger().warning(
                    "rust core feed disabled type=%s errors=%d",
                    type(error).__name__,
                    self.emit_errors,
                )
            return None

    def ensure_obligation(self, obligation_dict, timeout=2.0):
        """Idempotent schedule: 'armed', 'already_armed', 'rejected', None."""
        data = self.request("schedule_obligation", args=obligation_dict, timeout=timeout)
        if data is None:
            return None
        if data.get("ok"):
            return "armed"
        error = data.get("body", {}).get("error")
        if error == "duplicate_obligation":
            return "already_armed"
        get_runtime_logger().info("rust obligation rejected error=%s", error)
        return "rejected"

    def close(self):
        try:
            self._process.stop()
        except Exception as error:
            get_runtime_logger().warning(
                "rust core stop failed type=%s", type(error).__name__
            )
        if self._pid_path is not None:
            try:
                self._pid_path.unlink(missing_ok=True)
            except OSError:
                pass


def start_if_enabled(root=None, binary=None):
    """Spawn the shadow supervisor + feed when jarvis-core.toml opts in.

    Returns a RustCoreFeed, or None when the flag is off, the config is
    unreadable, no binary exists, or the process fails to start. The Python
    runtime is identical in every inactive case.
    """
    root = Path(root) if root is not None else _repo_root()
    config_path = root / CONFIG_NAME
    if not config_path.is_file():
        return None
    try:
        cfg = tomllib.loads(config_path.read_text(encoding="utf-8"))
    except (OSError, tomllib.TOMLDecodeError) as error:
        get_runtime_logger().warning(
            "rust core config unreadable type=%s", type(error).__name__
        )
        return None
    section = cfg.get("rust_core", {})
    if not section.get("enabled"):
        return None
    if binary is None:
        binary = adapter.find_binary(root)
    if binary is None:
        get_runtime_logger().info(
            "rust core enabled but no binary found; feed inactive"
        )
        return None
    data_dir = root / section.get("data_dir", "logs/rust-core")
    _reap_stale_supervisor(data_dir)
    try:
        process = adapter.RustCoreProcess(
            binary, config_path, data_dir=data_dir
        ).start()
    except Exception as error:
        get_runtime_logger().warning(
            "rust core spawn failed type=%s", type(error).__name__
        )
        return None
    pid_path = data_dir / PID_FILE
    try:
        data_dir.mkdir(parents=True, exist_ok=True)
        pid_path.write_text(str(process.pid), encoding="utf-8")
    except OSError:
        get_runtime_logger().warning("rust core pid file write failed")
    get_runtime_logger().info("rust core shadow feed active")
    return RustCoreFeed(process, pid_path)


def ensure_review_reminder(feed, now=None):
    """Register the daily pending-review obligation with the supervisor.

    Carries no user data — just a stable ID, scope, and recurrence. When it
    fires, Rust evaluates it through attention + policy and journals the
    shadow dispatch; nothing consequential runs until Stage C. Returns
    'armed' | 'already_armed' | 'rejected' | None (transport failure or a
    supervisor with ipc_requests off — the rollback path).
    """
    moment = now if now is not None else datetime.now(timezone.utc)
    result = feed.ensure_obligation(
        adapter.obligation(
            REVIEW_REMINDER_ID,
            moment + timedelta(seconds=REVIEW_REMINDER_RECURRENCE_S),
            scope="ops.obligation.run",
            origin="jarvis-app",
            priority=3,
            recurrence_seconds=REVIEW_REMINDER_RECURRENCE_S,
            missed_policy="run_once_on_recovery",
        )
    )
    if result is not None:
        get_runtime_logger().info("review reminder obligation %s", result)
    return result
