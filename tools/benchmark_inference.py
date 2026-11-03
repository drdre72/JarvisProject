"""Offline synthetic benchmark for the structured-intent inference path.

Run with .venv/Scripts/python.exe tools/benchmark_inference.py. No vault is read,
and only aggregate timings (not model output or prompts) are printed.
"""

import argparse
import re
import subprocess
import sys
import time
from pathlib import Path
from unittest.mock import patch


sys.path.insert(0, str(Path(__file__).resolve().parent.parent / "scripts"))
from reasoning import LlamaReasoning
from semantic_intent import INTENT_SCHEMA


TIMING = re.compile(
    r"(load time|prompt eval time|eval time|total time)\s*=\s*([\d.]+) ms"
    r"(?:\s*/\s*(\d+)\s*(?:tokens|runs))?"
)
STAMP = re.compile(r"^(\d+)\.(\d+)\.(\d+)\.(\d+)\s")
TURN = "User: Tell me how the imaginary workshop is doing.\nAssistant (generated): All systems nominal."
QUESTION = "And what should we check next?"


def timestamp_ms(line):
    match = STAMP.match(line)
    if not match:
        return None
    minutes, seconds, milliseconds, microseconds = map(int, match.groups())
    return (minutes * 60 + seconds) * 1000 + milliseconds + microseconds / 1000


def run_case(engine, label, context):
    captured = []
    real_run = subprocess.run

    def record(*args, **kwargs):
        result = real_run(*args, **kwargs)
        captured.append(result)
        return result

    start = time.perf_counter()
    with patch("reasoning.subprocess.run", side_effect=record):
        engine.generate_structured_response(
            QUESTION, INTENT_SCHEMA, max_tokens=64, context=context
        )
    wall_ms = (time.perf_counter() - start) * 1000
    result = captured[0]
    if result.returncode:
        raise RuntimeError(f"Local inference exited with code {result.returncode}")
    prompt = result.args[result.args.index("-p") + 1]
    timings = {
        name: (float(ms), int(tokens) if tokens else None)
        for name, ms, tokens in TIMING.findall(result.stderr + "\n" + result.stdout)
    }
    if not {"prompt eval time", "eval time", "total time"} <= timings.keys():
        raise RuntimeError("llama.cpp did not report prompt/generation timings")
    prompt_ms, prompt_tokens = timings["prompt eval time"]
    generation_ms, generated_tokens = timings["eval time"]
    total_ms, _ = timings["total time"]
    lines = result.stderr.splitlines()
    metadata_events = [
        timestamp_ms(line) for line in lines
        if "llama_model_loader: loaded meta data" in line
    ]
    context_events = [
        timestamp_ms(line) for line in lines
        if "llama_context: n_ctx_seq " in line
    ]
    # The binary initializes twice (preflight then real model context).
    # First metadata-to-last context is a model-setup estimate, not tensor I/O alone.
    # The residual also includes process launch and teardown. llama.cpp's
    # 'load time' counter overlaps prompt evaluation, so do not use it here.
    model_setup = (
        context_events[-1] - metadata_events[0]
        if metadata_events and context_events else None
    )
    model_setup_label = f"{model_setup:.0f}" if model_setup is not None else "unavailable"
    print(
        f"{label}: prompt_chars={len(prompt)} prompt_tokens={prompt_tokens} "
        f"model_setup_ms={model_setup_label} "
        f"startup_load_envelope_ms={wall_ms - total_ms:.0f} "
        f"prompt_eval_ms={prompt_ms:.0f} generation_ms={generation_ms:.0f} "
        f"generated_runs={generated_tokens} wall_ms={wall_ms:.0f}"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repeats", type=int, default=1)
    args = parser.parse_args()
    if not 1 <= args.repeats <= 5:
        parser.error("--repeats must be between 1 and 5")
    engine = LlamaReasoning()
    synthetic_long = (TURN + "\n") * 40
    synthetic_short = (TURN + "\n") * 2
    for _ in range(args.repeats):
        for label, context in (
            ("no_history", None),
            ("long_history", synthetic_long),
            ("short_history", synthetic_short),
        ):
            run_case(engine, label, context)


if __name__ == "__main__":
    main()
