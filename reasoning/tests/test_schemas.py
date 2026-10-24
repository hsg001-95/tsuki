import json
from tsuki_agent.schemas import (
    ActionState,
    ActionTemplate,
    CitationItem,
    DiagnosisReport,
    EvidenceItem,
    RemediationIntent,
)


def test_remediation_intent_serialization():
    intent = RemediationIntent(
        intent_id="intent-001",
        template=ActionTemplate.THROTTLE_MEMORY,
        target_cgroup="/user.slice/user-1000.slice/app.slice/build.service",
        params={"memory_high_bytes": 1073741824},
        rationale="Working set memory thrash detected under parallel build",
        evidence_ids=["psi_mem_some", "refault_spike"],
        ttl_seconds=180,
    )

    data = json.loads(intent.model_dump_json())
    assert data["intent_id"] == "intent-001"
    assert data["template"] == "throttle_memory"
    assert data["ttl_seconds"] == 180


def test_diagnosis_report_with_citations():
    report = DiagnosisReport(
        incident_id="inc-99",
        fault_type="memory_thrash",
        confidence=0.94,
        evidence=[
            EvidenceItem(
                metric="memory_some_avg10",
                window_sec=60,
                value=72.4,
                unit="percent",
                cgroup="/build.service",
            )
        ],
        citations=[
            CitationItem(
                chunk_id="kernel_cgroup_v2#memory-high",
                source="Documentation/admin-guide/cgroup-v2.rst",
                section="Memory Interface Files",
                title="memory.high throttling mechanism",
                snippet="memory.high is the main mechanism to throttle allocations before OOM.",
            )
        ],
        candidate_intents=[],
    )

    assert report.confidence > 0.9
    assert len(report.citations) == 1
    assert report.citations[0].chunk_id.startswith("kernel_cgroup_v2")
