"""
Tsuki Reasoning Plane: Core Pydantic schemas.
Guarantees strict type safety and schema parity with tsuki-common (Rust).
"""

from enum import Enum
from typing import Any, Dict, List, Optional
from pydantic import BaseModel, Field


class ActionTemplate(str, Enum):
    THROTTLE_MEMORY = "throttle_memory"
    REWEIGHT_CPU = "reweight_cpu"
    FREEZE_CGROUP = "freeze_cgroup"


class ActionState(str, Enum):
    PROPOSED = "proposed"
    VALIDATED = "validated"
    AWAITING_APPROVAL = "awaiting_approval"
    APPLIED = "applied"
    OBSERVING = "observing"
    COMMITTED = "committed"
    ROLLED_BACK = "rolled_back"
    REJECTED = "rejected"
    EXPIRED = "expired"


class RemediationIntent(BaseModel):
    intent_id: str
    template: ActionTemplate
    target_cgroup: str
    params: Dict[str, Any] = Field(default_factory=dict)
    rationale: str
    evidence_ids: List[str] = Field(default_factory=list)
    ttl_seconds: int = 300


class EvidenceItem(BaseModel):
    metric: str
    window_sec: int
    value: float
    unit: str
    cgroup: Optional[str] = None


class CitationItem(BaseModel):
    chunk_id: str
    source: str
    section: str
    title: str
    snippet: str


class DiagnosisReport(BaseModel):
    incident_id: str
    fault_type: str
    confidence: float
    evidence: List[EvidenceItem] = Field(default_factory=list)
    citations: List[CitationItem] = Field(default_factory=list)
    candidate_intents: List[RemediationIntent] = Field(default_factory=list)
