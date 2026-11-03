"""Synthetic timing parser checks; no model or vault is loaded."""

import unittest

from benchmark_inference import TIMING, timestamp_ms


class BenchmarkTimingTests(unittest.TestCase):
    def test_verbose_timestamps_measure_model_setup_interval(self):
        metadata = timestamp_ms(
            "0.00.080.227 I llama_model_loader: loaded meta data"
        )
        context = timestamp_ms("0.03.316.330 I llama_context: n_ctx_seq = 4096")
        self.assertAlmostEqual(context - metadata, 3236.103)
        self.assertIsNone(timestamp_ms("no timestamp"))

    def test_performance_counters_are_distinct(self):
        output = (
            "I common_perf_print: load time = 900.00 ms\n"
            "I common_perf_print: prompt eval time = 900.00 ms / 20 tokens\n"
            "I common_perf_print: eval time = 250.00 ms / 2 runs\n"
            "I common_perf_print: total time = 1150.00 ms / 22 tokens"
        )
        timings = {name: (float(ms), tokens) for name, ms, tokens in TIMING.findall(output)}
        self.assertEqual(timings["prompt eval time"], (900.0, "20"))
        self.assertEqual(timings["eval time"], (250.0, "2"))
        self.assertEqual(timings["total time"], (1150.0, "22"))


if __name__ == "__main__":
    unittest.main()
