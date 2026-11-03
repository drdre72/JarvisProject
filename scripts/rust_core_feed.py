"""Optional Stage A shadow feed: emit real turn events to jarvis-core.

Active only when the repo-root ``jarvis-core.toml`` parses with
``[rust_core] enabled = true`` and a built binary exists. Every send is
bounded and failure-isolated -- a dead or slow supervisor never breaks a
turn. Payloads are operational metadata only (input kind, outcome codes,
timings): the journal persists them, so transcripts and prompts never
belong on this channel.
"""

import threading
import tomllib
from pathlib import Path

import rust_core_adapter as adapter
from runtime_log import get_runtime_logger

CONFIG_NAME = "jarvis-core.toml"
_MAX_SEND_FAILURES = 8


def _repo_root():
    return Path(__file__).resolve().parent.parent


class RustCoreFeed:
    """Failure-isolated event emitter over one RustCoreProcess."""

    def __init__(self, process):
        self._process = process
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

    def close(self):
        try:
            self._process.stop()
        except Exception as error:
            get_runtime_logger().warning(
                "rust core stop failed type=%s", type(error).__name__
            )


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
    try:
        process = adapter.RustCoreProcess(
            binary, config_path, data_dir=data_dir
        ).start()
    except Exception as error:
        get_runtime_logger().warning(
            "rust core spawn failed type=%s", type(error).__name__
        )
        return None
    get_runtime_logger().info("rust core shadow feed active")
    return RustCoreFeed(process)
