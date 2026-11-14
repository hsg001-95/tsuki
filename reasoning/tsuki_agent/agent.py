"""
Tsuki Diagnosis Agent.
Implements the tool-using reasoning loop:
  1. Inspects evidence snapshots from the guard plane
  2. Classifies root cause failure modes
  3. Gathers grounded documentation evidence via hybrid RAG
  4. Generates structured DiagnosisReport with citations and candidate remediation intents
"""

from typing import Dict, List, Optional
import uuid

from tsuki_agent.citation import CitationValidator
from tsuki_agent.classifier import FaultClassifier
from tsuki_agent.schemas import (
    ActionTemplate,
    CitationItem,
    DiagnosisReport,
    EvidenceItem,
    RemediationIntent,
)
from tsuki_agent.tools import AgentTools


class DiagnosisAgent:
    def __init__(
        self,
        tools: Optional[AgentTools] = None,
        validator: Optional[CitationValidator] = None,
        classifier: Optional[FaultClassifier] = None,
    ):
        self.tools = tools or AgentTools()
        self.validator = validator or CitationValidator()
        self.classifier = classifier or FaultClassifier()

    def diagnose_snapshot(self, snapshot: Dict[str, any]) -> DiagnosisReport:
        incident_id = snapshot.get("snapshot_id", f"inc-{uuid.uuid4().hex[:8]}")
        samples = snapshot.get("samples", [])
        if not samples:
            return DiagnosisReport(
                incident_id=incident_id,
                fault_type="unknown",
                confidence=0.0,
                evidence=[],
                citations=[],
                candidate_intents=[],
            )

        latest_sample = samples[-1]
        fault_type, confidence = self.classifier.classify(latest_sample)

        # 1. Gather evidence items
        pressure = latest_sample.get("pressure", {})
        mem = pressure.get("memory", {})
        cpu = pressure.get("cpu", {})
        vmstat = latest_sample.get("vmstat", {})

        evidence = []
        if mem.get("some_avg10", 0.0) > 0:
            evidence.append(
                EvidenceItem(
                    metric="psi_memory_some_avg10",
                    window_sec=10,
                    value=round(mem.get("some_avg10", 0.0), 2),
                    unit="percent",
                    cgroup=snapshot.get("primary_cgroup"),
                )
            )
        if vmstat.get("workingset_refault", 0) > 0:
            evidence.append(
                EvidenceItem(
                    metric="workingset_refault",
                    window_sec=1,
                    value=float(vmstat.get("workingset_refault", 0)),
                    unit="events/sec",
                )
            )
        if cpu.get("some_avg10", 0.0) > 0:
            evidence.append(
                EvidenceItem(
                    metric="psi_cpu_some_avg10",
                    window_sec=10,
                    value=round(cpu.get("some_avg10", 0.0), 2),
                    unit="percent",
                )
            )

        # 2. Retrieve documentation citations
        query = f"{fault_type} cgroup v2 throttling kernel pressure"
        citations = self.tools.search_docs(query, k=3)

        # 3. Propose candidate remediation intents
        candidate_intents = []
        primary_cg = snapshot.get("primary_cgroup")

        # Check protected list
        protected = self.tools.protected_list()
        is_protected = primary_cg in protected.get("cgroups", [])

        if primary_cg and not is_protected:
            if fault_type == "memory_thrash":
                # Propose memory.high throttle to 85% of current usage
                candidate_intents.append(
                    RemediationIntent(
                        intent_id=f"intent-{uuid.uuid4().hex[:6]}",
                        template=ActionTemplate.THROTTLE_MEMORY,
                        target_cgroup=primary_cg,
                        params={"ratio_of_current": 0.85},
                        rationale="Reclaim memory proactively via memory.high without OOM kill",
                        evidence_ids=["psi_memory_some_avg10", "workingset_refault"],
                        ttl_seconds=300,
                    )
                )
            elif fault_type == "cpu_contention":
                # Propose cpu.weight lowering to 20
                candidate_intents.append(
                    RemediationIntent(
                        intent_id=f"intent-{uuid.uuid4().hex[:6]}",
                        template=ActionTemplate.REWEIGHT_CPU,
                        target_cgroup=primary_cg,
                        params={"cpu_weight": 20},
                        rationale="Lower CPU priority of noisy neighbor to relieve run-queue saturation",
                        evidence_ids=["psi_cpu_some_avg10"],
                        ttl_seconds=300,
                    )
                )

        # 4. Construct and verify report
        report = DiagnosisReport(
            incident_id=incident_id,
            fault_type=fault_type,
            confidence=confidence,
            evidence=evidence,
            citations=citations,
            candidate_intents=candidate_intents,
        )

        return self.validator.verify_report(report)
