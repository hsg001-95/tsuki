import os
import pytest
from probe import ResponsivenessProbe
from dataset import FaultBenchDataset
from harness import ScenarioHarness


def test_responsiveness_probe_basic():
    probe = ResponsivenessProbe(interval_sec=0.01, window_sec=0.5, freeze_threshold_ms=100.0)
    probe.start()
    import time
    time.sleep(0.1)
    records = probe.stop()

    assert len(records) > 0
    summary = probe.get_summary()
    assert summary["samples"] == len(records)
    assert summary["max_p99_ms"] >= 0.0


def test_scenario_harness_config_loading():
    harness = ScenarioHarness("bench/scenarios/memory_thrash/service_leak.yaml")
    assert harness.scenario_id == "memory_thrash_service_leak"
    assert harness.category == "memory_thrash"
    assert harness.total_duration == 15.0


def test_dataset_leave_one_family_out_split():
    ds = FaultBenchDataset("bench/data")
    summary = ds.summary()
    assert summary["scenarios"] >= 3

    # Test splitting by family 'build'
    X_train, y_train, X_test, y_test = ds.get_leave_one_family_out_split("build")
    assert len(X_train) > 0
    assert len(X_test) > 0
    assert len(X_train) + len(X_test) == summary["total_samples"]
