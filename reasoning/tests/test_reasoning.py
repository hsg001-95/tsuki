import os
import sys
import pytest
from fastapi.testclient import TestClient

sys.path.insert(0, os.path.abspath("reasoning"))

from tsuki_agent.agent import DiagnosisAgent
from tsuki_agent.citation import CitationValidator
from tsuki_agent.classifier import FaultClassifier
from tsuki_agent.schemas import CitationItem
from tsuki_agent.tools import AgentTools
from tsuki_api.app import app
from tsuki_rag.indexer import parse_markdown_corpus
from tsuki_rag.retriever import HybridRetriever


def test_corpus_parsing():
    corpus_dir = "reasoning/tsuki_rag/corpus"
    chunks = parse_markdown_corpus(corpus_dir)
    assert len(chunks) >= 5
    ids = [c["chunk_id"] for c in chunks]
    assert "kernel_cgroup_v2#memory-high" in ids
    assert "kernel_psi#some-vs-full" in ids


def test_hybrid_retriever_search():
    retriever = HybridRetriever(db_path="reasoning/tsuki_rag/index.db")
    results = retriever.search("memory high throttling", k=3)
    assert len(results) > 0
    assert any("memory-high" in r.chunk_id for r in results)


def test_citation_validator_filtering():
    validator = CitationValidator(db_path="reasoning/tsuki_rag/index.db")
    citations = [
        CitationItem(
            chunk_id="kernel_cgroup_v2#memory-high",
            source="test",
            section="test",
            title="test",
            snippet="test",
        ),
        CitationItem(
            chunk_id="hallucinated_chunk_id_12345",
            source="fake",
            section="fake",
            title="fake",
            snippet="fake",
        ),
    ]

    verified = validator.validate_citations(citations)
    assert len(verified) == 1
    assert verified[0].chunk_id == "kernel_cgroup_v2#memory-high"


def test_fault_classifier():
    classifier = FaultClassifier()
    sample = {
        "pressure": {
            "memory": {"some_avg10": 35.0, "full_avg10": 15.0},
            "cpu": {"some_avg10": 5.0},
            "io": {"full_avg10": 1.0},
        },
        "vmstat": {"workingset_refault": 1200},
    }

    fault_type, conf = classifier.classify(sample)
    assert fault_type == "memory_thrash"
    assert conf >= 0.7


def test_diagnosis_agent_report_generation():
    agent = DiagnosisAgent()
    sample_snapshot = {
        "snapshot_id": "snap-test-01",
        "primary_cgroup": "/user.slice/app.slice/worker.service",
        "samples": [
            {
                "pressure": {
                    "memory": {"some_avg10": 28.0, "full_avg10": 12.0},
                    "cpu": {"some_avg10": 5.0},
                    "io": {"some_avg10": 1.0},
                },
                "vmstat": {"workingset_refault": 900},
            }
        ],
    }

    report = agent.diagnose_snapshot(sample_snapshot)
    assert report.fault_type == "memory_thrash"
    assert report.confidence > 0.5
    assert report.summary is not None
    assert "MEMORY_THRASH" in report.summary
    assert len(report.citations) > 0
    assert all("kernel_" in c.chunk_id or "systemd" in c.chunk_id or "os_" in c.chunk_id for c in report.citations)
    assert len(report.candidate_intents) > 0
    assert report.candidate_intents[0].target_cgroup == "/user.slice/app.slice/worker.service"


def test_retriever_special_characters():
    retriever = HybridRetriever(db_path="reasoning/tsuki_rag/index.db")
    results = retriever.search('""**?? memory::: NEAR/2 throttling', k=2)
    assert isinstance(results, list)


def test_fastapi_endpoints():
    client = TestClient(app)
    resp = client.get("/api/status")
    assert resp.status_code == 200
    assert resp.json()["status"] == "online"

    panic_resp = client.post("/api/panic")
    assert panic_resp.status_code == 200
    assert panic_resp.json()["status"] == "panic_engaged"
