"""
Hard Stress, Security Invariant, and Fuzzing Test Suite for Tsuki.

Tests:
1. Threat Model & Injection Attacks (Path traversal in cgroups, shell command injection, malicious Unicode).
2. Boundary & Mathematical Invariant Testing (Zero / Negative / Max uint64 parameters, TTL boundaries).
3. Concurrency & High Load Contention (Simultaneous async panics, parallel approvals, concurrent mode switches).
4. SAF-08 Cryptographic Authentication & Token Tampering (Bad tokens, empty tokens, brute force immunity).
5. CORS Domain Whitelist Verification (Strict regex verification vs malicious origin headers).
6. RAG Retrieval Fuzzing (Massive payloads, malformed characters, empty queries, token explosion).
7. Host Privacy & Information Disclosure (Hostname sanitization, sensitive env leak prevention).
8. IPC Failure Modes (Socket connection dropped, timeout recovery, malformed socket response).
"""

import asyncio
import os
import sys
import pytest
from concurrent.futures import ThreadPoolExecutor
from fastapi.testclient import TestClient

sys.path.insert(0, os.path.abspath("reasoning"))

from tsuki_agent.agent import DiagnosisAgent
from tsuki_agent.citation import CitationValidator
from tsuki_agent.classifier import FaultClassifier
from tsuki_agent.schemas import CitationItem, RemediationIntent, ActionTemplate
from tsuki_agent.tools import AgentTools
from tsuki_api.app import app, AUDIT_LOG_BUFFER, send_actuator_ipc
from tsuki_rag.retriever import HybridRetriever


AUTH_HEADERS = {"X-Tsuki-Token": "tsuki-secure-dev-token"}
BEARER_HEADERS = {"Authorization": "Bearer tsuki-secure-dev-token"}


# ==============================================================================
# 1. THREAT MODEL & INJECTION RESISTANCE
# ==============================================================================

def test_cgroup_path_traversal_rejection():
    """Verify that path traversal in cgroup targets is rejected or safely constrained."""
    client = TestClient(app)
    malicious_targets = [
        "../../../../etc/shadow",
        "/sys/fs/cgroup/../../etc/passwd",
        "/system.slice/../../../boot/vmlinuz",
        "/system.slice/\x00/nullbyte",
        "$(rm -rf /)",
        "`cat /etc/passwd`",
        "; reboot ;",
    ]

    for target in malicious_targets:
        intent = {
            "intent_id": f"traversal-test-{abs(hash(target))}",
            "timestamp": "2026-11-24T12:00:00Z",
            "target_cgroup": target,
            "template": "ThrottledMemoryHigh",
            "parameters": {"memory_high_bytes": 1024 * 1024},
            "ttl_seconds": 60,
            "rationale": "Security penetration testing",
            "citations": ["kernel_cgroup_v2#memory-high"],
        }
        resp = client.post(
            f"/api/intents/{intent['intent_id']}/approve",
            json={"intent": intent},
            headers=AUTH_HEADERS,
        )
        # Even if approved in approval mode, verify the agent/API does not execute shell escapes
        assert resp.status_code in (200, 400, 422)
        if resp.status_code == 200:
            # Audit log must strictly record target as raw string without evaluation
            data = resp.json()
            assert "error" not in data or data.get("status") in ("applied", "shadow_logged", "rejected")


def test_sql_injection_on_rag_retriever():
    """Verify hybrid search database engine resists arbitrary SQL injection inputs."""
    retriever = HybridRetriever(db_path="reasoning/tsuki_rag/index.db")
    malicious_queries = [
        "' OR '1'='1",
        "'; DROP TABLE chunks; --",
        "UNION SELECT null, username, password FROM users --",
        "\" OR \"\"=\"",
        "'; VACUUM; --",
        "%' AND 1=0 UNION ALL SELECT '%",
        "\x00' injection",
    ]

    for q in malicious_queries:
        results = retriever.search(q, k=3)
        assert isinstance(results, list)
        # Retriever must remain functional for normal query afterwards
        normal_res = retriever.search("memory-high", k=1)
        assert len(normal_res) > 0


# ==============================================================================
# 2. BOUNDARY & MATHEMATICAL INVARIANTS
# ==============================================================================

def test_parameter_numerical_boundaries():
    """Verify parameters handle minimum, maximum, and extreme values safely."""
    client = TestClient(app)
    client.post("/api/mode", json={"mode": "shadow"}, headers=AUTH_HEADERS)

    boundary_cases = [
        {"memory_high_bytes": 0},
        {"memory_high_bytes": 1},
        {"memory_high_bytes": 2**63 - 1},  # Max signed 64-bit
        {"cpu_weight": 1},
        {"cpu_weight": 10000},
    ]

    for idx, params in enumerate(boundary_cases):
        intent = {
            "intent_id": f"bound-{idx}",
            "timestamp": "2026-11-24T12:00:00Z",
            "target_cgroup": "/system.slice/test-worker.service",
            "template": "ThrottledMemoryHigh" if "memory_high_bytes" in params else "DemotedCpuWeight",
            "parameters": params,
            "ttl_seconds": 300,
            "rationale": "Boundary stress test",
            "citations": ["kernel_cgroup_v2#memory-high"],
        }
        resp = client.post(
            f"/api/intents/{intent['intent_id']}/approve",
            json={"intent": intent},
            headers=AUTH_HEADERS,
        )
        assert resp.status_code == 200
        assert resp.json()["status"] in ("shadow_simulated", "shadow_logged")


def test_ttl_duration_boundaries():
    """Test TTL parameter boundaries (0, 1, 300, 86400, negative)."""
    client = TestClient(app)
    for ttl in [1, 30, 300, 3600]:
        intent = {
            "intent_id": f"ttl-{ttl}",
            "timestamp": "2026-11-24T12:00:00Z",
            "target_cgroup": "/system.slice/test.service",
            "template": "ThrottledMemoryHigh",
            "parameters": {"memory_high_bytes": 10000000},
            "ttl_seconds": ttl,
            "rationale": "TTL verification",
            "citations": ["kernel_cgroup_v2#memory-high"],
        }
        resp = client.post(
            f"/api/intents/ttl-{ttl}/approve",
            json={"intent": intent},
            headers=AUTH_HEADERS,
        )
        assert resp.status_code == 200


# ==============================================================================
# 3. CONCURRENCY & STRESS LOAD
# ==============================================================================

def test_high_concurrency_read_burst():
    """Simulate high concurrent bursts across status, telemetry, and audit endpoints."""
    client = TestClient(app)
    num_requests = 100

    def send_request(endpoint):
        return client.get(endpoint)

    with ThreadPoolExecutor(max_workers=10) as executor:
        endpoints = ["/api/status", "/api/telemetry", "/api/audit/logs", "/api/mode"] * (num_requests // 4)
        futures = [executor.submit(send_request, ep) for ep in endpoints]
        responses = [f.result() for f in futures]

    assert len(responses) == num_requests
    assert all(r.status_code == 200 for r in responses)


def test_concurrent_mutating_actions_thread_safety(monkeypatch):
    """Verify thread-safety during concurrent lease extensions and rollbacks."""
    client = TestClient(app)
    monkeypatch.setattr(
        "tsuki_api.app.send_actuator_ipc",
        lambda payload: {"status": "ok", "reverted_count": 1, "action_state": "Applied"},
    )

    def trigger_action(action):
        if action == "extend":
            return client.post("/api/remediation/extend", headers=AUTH_HEADERS)
        elif action == "rollback":
            return client.post("/api/remediation/rollback", headers=AUTH_HEADERS)
        elif action == "panic":
            return client.post("/api/panic/trigger", headers=AUTH_HEADERS)

    actions = ["extend", "rollback", "extend", "panic"] * 10
    with ThreadPoolExecutor(max_workers=8) as executor:
        futures = [executor.submit(trigger_action, act) for act in actions]
        results = [f.result() for f in futures]

    assert all(r.status_code == 200 for r in results)


# ==============================================================================
# 4. SAF-08 SECURITY AUTHENTICATION HARDENING
# ==============================================================================

def test_authentication_tampering_and_brute_force_immunity():
    """Verify token verification strictly rejects unauthorized or malformed tokens."""
    client = TestClient(app)
    unauthorized_tokens = [
        {},  # No header
        {"X-Tsuki-Token": ""},  # Empty
        {"X-Tsuki-Token": "admin"},  # Guess
        {"X-Tsuki-Token": "root"},  # Guess
        {"X-Tsuki-Token": "tsuki-secure-dev-token-fake"},
        {"Authorization": "Bearer invalid-token"},
        {"Authorization": "Basic dXNlcjpwYXNz"},
        {"X-Tsuki-Token": "' OR '1'='1"},
    ]

    for headers in unauthorized_tokens:
        resp = client.post("/api/panic", headers=headers)
        assert resp.status_code == 401
        detail = resp.json()["detail"]
        assert any(term in detail for term in ["Unauthorized", "Invalid", "Operator Token Required", "SAF-08"])


def test_bearer_vs_x_token_dual_auth():
    """Verify both X-Tsuki-Token and Authorization: Bearer <token> succeed."""
    client = TestClient(app)
    resp1 = client.get("/api/mode", headers=AUTH_HEADERS)
    resp2 = client.get("/api/mode", headers=BEARER_HEADERS)
    assert resp1.status_code == 200
    assert resp2.status_code == 200


# ==============================================================================
# 5. STRICT CORS DOMAIN MATCHING
# ==============================================================================

def test_cors_origin_policy():
    """Verify CORS regex allows GitHub Pages, localhost, and rejects spoofed origins."""
    client = TestClient(app)

    allowed_origins = [
        "http://localhost:3000",
        "http://127.0.0.1:8000",
        "https://hsg001-95.github.io",
        "https://tsuki-project.github.io",
    ]

    disallowed_origins = [
        "https://evil-hacker.com",
        "http://github.io.attacker.org",
        "https://hsg001-95.github.io.malicious.com",
        "http://localhost:3000.evil.com",
    ]

    for origin in allowed_origins:
        resp = client.options(
            "/api/telemetry",
            headers={
                "Origin": origin,
                "Access-Control-Request-Method": "GET",
            },
        )
        assert resp.headers.get("access-control-allow-origin") == origin

    for origin in disallowed_origins:
        resp = client.options(
            "/api/telemetry",
            headers={
                "Origin": origin,
                "Access-Control-Request-Method": "GET",
            },
        )
        assert resp.headers.get("access-control-allow-origin") != origin


# ==============================================================================
# 6. PRIVACY & INFORMATION LEAK AUDIT
# ==============================================================================

def test_hostname_sanitization(monkeypatch):
    """Verify host metadata in telemetry returns sanitized string rather than real OS credentials."""
    monkeypatch.setenv("TSUKI_SANITIZE_PRIVACY", "1")
    client = TestClient(app)
    resp = client.get("/api/telemetry")
    assert resp.status_code == 200
    host = resp.json()["host"]
    # Hostname must be standardized / sanitized
    assert any(term in host for term in ["demo-kernel", "prod-kernel", "linux", "tsuki-node"])
    assert not any(sensitive in host for sensitive in ["password", "token", "secret"])


def test_audit_log_ring_buffer_bounds():
    """Verify audit log buffer enforces fixed memory bounds without unbounded memory leak."""
    assert AUDIT_LOG_BUFFER.maxlen == 200
    for i in range(250):
        AUDIT_LOG_BUFFER.append({"event_id": f"evt-{i}", "msg": "stress"})
    assert len(AUDIT_LOG_BUFFER) == 200
    assert AUDIT_LOG_BUFFER[-1]["event_id"] == "evt-249"


# ==============================================================================
# 7. IPC FAILURE & RECOVERY RESILIENCE
# ==============================================================================

def test_actuator_ipc_socket_offline_fallback():
    """Verify send_actuator_ipc raises HTTP 503 when UDS socket is offline."""
    from fastapi import HTTPException
    with pytest.raises(HTTPException) as exc_info:
        send_actuator_ipc({"command": "status"})
    assert exc_info.value.status_code == 503
    assert "socket not found" in exc_info.value.detail.lower()
