"""
Tsuki Reasoning Plane: FastAPI Service.
Serves dashboard endpoints, status telemetry, snapshot retrieval,
diagnosis reports, and authenticated operator approval & emergency panic controls.
"""

import glob
import json
import logging
import os
import socket
import sys
import time
from pathlib import Path
from typing import Any, Dict, List, Optional
from fastapi import Depends, FastAPI, Header, HTTPException, Request, Security, status
from fastapi.security import HTTPAuthorizationCredentials, HTTPBearer
from pydantic import BaseModel

sys.path.insert(0, os.path.abspath("reasoning"))
from tsuki_agent.agent import DiagnosisAgent
from tsuki_agent.schemas import DiagnosisReport

logging.basicConfig(level=logging.INFO, format="%(asctime)s [%(levelname)s] %(name)s: %(message)s")
logger = logging.getLogger("tsuki.api")

app = FastAPI(
    title="Tsuki Reasoning API",
    version="0.1.0",
    description="Safety-governed Linux kernel diagnosis and remediation control plane",
)

agent = DiagnosisAgent()

SNAPSHOT_DIR = Path(os.environ.get("TSUKI_SNAPSHOT_DIR", "/var/run/tsuki/snapshots"))
PENDING_DIR = Path(os.environ.get("TSUKI_PENDING_INTENTS_DIR", "/var/run/tsuki/pending_intents"))
ACTUATOR_SOCKET = Path(os.environ.get("TSUKI_SOCKET_PATH", "/var/run/tsuki/actuator.sock"))
API_TOKEN = os.environ.get("TSUKI_API_TOKEN", "tsuki-secure-dev-token")

security_bearer = HTTPBearer(auto_error=False)


# -----------------------------------------------------------------------------
# Security & Authorization Middleware (SAF-02 / Issue 9)
# -----------------------------------------------------------------------------
def verify_admin_auth(
    request: Request,
    auth: Optional[HTTPAuthorizationCredentials] = Security(security_bearer),
    x_tsuki_token: Optional[str] = Header(None, alias="X-Tsuki-Token"),
) -> str:
    """Validate Bearer token or X-Tsuki-Token header for mutating actions."""
    token = None
    if auth and auth.credentials:
        token = auth.credentials
    elif x_tsuki_token:
        token = x_tsuki_token

    if not token or token != API_TOKEN:
        logger.warning(
            "AUDIT: Unauthorized API control attempt from %s on %s %s",
            request.client.host if request.client else "unknown",
            request.method,
            request.url.path,
        )
        raise HTTPException(
            status_code=status.HTTP_401_UNAUTHORIZED,
            detail="Invalid or missing Tsuki API authentication token (X-Tsuki-Token or Bearer)",
            headers={"WWW-Authenticate": "Bearer"},
        )

    return token


# -----------------------------------------------------------------------------
# Actuator Unix Domain Socket IPC (Issues 1, 2, 3)
# -----------------------------------------------------------------------------
def send_actuator_ipc(payload: Dict[str, Any]) -> Dict[str, Any]:
    """Send command to tsuki-actuator over Unix Domain Socket."""
    if not ACTUATOR_SOCKET.exists():
        raise HTTPException(
            status_code=status.HTTP_503_SERVICE_UNAVAILABLE,
            detail=f"Tsuki Actuator UDS socket not found at {ACTUATOR_SOCKET}. Is tsuki-actuator daemon running?",
        )

    try:
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(5.0)
            client.connect(str(ACTUATOR_SOCKET))
            wire_msg = json.dumps(payload) + "\n"
            client.sendall(wire_msg.encode("utf-8"))

            chunks = []
            while True:
                chunk = client.recv(4096)
                if not chunk:
                    break
                chunks.append(chunk)
                if b"\n" in chunk:
                    break

            raw = b"".join(chunks).decode("utf-8").strip()
            if not raw:
                raise HTTPException(
                    status_code=status.HTTP_502_BAD_GATEWAY,
                    detail="Empty response received from actuator daemon",
                )
            res = json.loads(raw)
            return res
    except socket.timeout:
        raise HTTPException(
            status_code=status.HTTP_504_GATEWAY_TIMEOUT,
            detail="Actuator daemon timed out processing command",
        )
    except HTTPException:
        raise
    except Exception as exc:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Actuator IPC communication failure: {exc}",
        )


# -----------------------------------------------------------------------------
# Request & Response Schemas
# -----------------------------------------------------------------------------
class ApprovePayload(BaseModel):
    intent: Optional[Dict[str, Any]] = None
    operator_note: Optional[str] = "Operator approved via Tsuki API"


class RejectPayload(BaseModel):
    reason: Optional[str] = "Rejected by operator"


# -----------------------------------------------------------------------------
# Public Read-Only Observability Endpoints
# -----------------------------------------------------------------------------
@app.get("/api/status")
def get_status() -> Dict[str, Any]:
    return {
        "status": "online",
        "guard_plane": "active",
        "reasoning_plane": "active",
        "mode": os.environ.get("TSUKI_MODE", "shadow"),
        "version": "0.1.0",
        "socket_path": str(ACTUATOR_SOCKET),
        "snapshot_dir": str(SNAPSHOT_DIR),
    }


@app.get("/api/actuator/status")
def get_actuator_status() -> Dict[str, Any]:
    """Query live actuator daemon status and panic latch over UDS."""
    return send_actuator_ipc({"command": "status"})


@app.get("/api/snapshots")
def list_snapshots() -> List[str]:
    files = glob.glob(os.path.join(str(SNAPSHOT_DIR), "*.json"))
    return [os.path.splitext(os.path.basename(f))[0] for f in sorted(files, reverse=True)]


@app.get("/api/snapshots/{snapshot_id}")
def get_snapshot(snapshot_id: str) -> Dict[str, Any]:
    try:
        snap = agent.tools.get_snapshot(snapshot_id)
    except Exception as exc:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error reading snapshot '{snapshot_id}': {exc}",
        )
    if not snap:
        raise HTTPException(status_code=404, detail=f"Snapshot '{snapshot_id}' not found")
    return snap


@app.post("/api/diagnose/{snapshot_id}", response_model=DiagnosisReport)
def diagnose_snapshot(snapshot_id: str):
    try:
        snap = agent.tools.get_snapshot(snapshot_id)
    except Exception as exc:
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Error reading snapshot '{snapshot_id}': {exc}",
        )
    if not snap:
        raise HTTPException(status_code=404, detail=f"Snapshot '{snapshot_id}' not found")
    report = agent.diagnose_snapshot(snap)
    return report


@app.get("/api/intents/pending")
def list_pending_intents() -> List[Dict[str, Any]]:
    """List remediation intents awaiting operator approval."""
    if not PENDING_DIR.exists():
        return []

    pending = []
    for f in sorted(PENDING_DIR.glob("*.json")):
        try:
            with open(f, "r", encoding="utf-8") as fp:
                pending.append(json.load(fp))
        except Exception as exc:
            logger.error("Error reading pending intent file %s: %s", f, exc)
    return pending


# -----------------------------------------------------------------------------
# Authenticated Remediation & Emergency Endpoints (Issues 2, 3, 9)
# -----------------------------------------------------------------------------
@app.post("/api/intents/{intent_id}/approve")
def approve_intent(
    intent_id: str,
    payload: Optional[ApprovePayload] = None,
    request: Request = None,
    _auth: str = Depends(verify_admin_auth),
):
    """Approve a pending or provided remediation intent and forward to actuator daemon."""
    intent_to_apply = None

    if payload and payload.intent:
        intent_to_apply = payload.intent
    else:
        pending_file = PENDING_DIR / f"{intent_id}.json"
        if pending_file.exists():
            try:
                with open(pending_file, "r", encoding="utf-8") as fp:
                    intent_to_apply = json.load(fp)
            except Exception as exc:
                raise HTTPException(
                    status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
                    detail=f"Failed to read pending intent file {pending_file}: {exc}",
                )

    if not intent_to_apply:
        raise HTTPException(
            status_code=status.HTTP_404_NOT_FOUND,
            detail=f"Intent '{intent_id}' not found in pending queue and not provided in request body",
        )

    logger.info(
        "AUDIT: Operator approved intent [%s] from %s. Note: %s",
        intent_id,
        request.client.host if request and request.client else "unknown",
        payload.operator_note if payload else "N/A",
    )

    # Forward to actuator daemon via UDS
    res = send_actuator_ipc({"command": "apply", "intent": intent_to_apply})

    if res.get("status") == "error":
        logger.error("Actuator daemon rejected approved intent [%s]: %s", intent_id, res.get("error"))
        raise HTTPException(
            status_code=status.HTTP_400_BAD_REQUEST,
            detail=f"Actuator validation or execution rejected intent: {res.get('error')}",
        )

    # Clean up staged pending file on successful application
    pending_file = PENDING_DIR / f"{intent_id}.json"
    if pending_file.exists():
        try:
            pending_file.unlink()
        except Exception as e:
            logger.warning("Failed to remove applied pending intent file %s: %s", pending_file, e)

    return {
        "intent_id": intent_id,
        "status": "applied",
        "action_state": res.get("action_state"),
        "daemon_response": res,
    }


@app.post("/api/intents/{intent_id}/reject")
def reject_intent(
    intent_id: str,
    payload: Optional[RejectPayload] = None,
    request: Request = None,
    _auth: str = Depends(verify_admin_auth),
):
    """Reject and purge a staged remediation intent."""
    pending_file = PENDING_DIR / f"{intent_id}.json"
    if not pending_file.exists():
        raise HTTPException(status_code=404, detail=f"Pending intent '{intent_id}' not found")

    logger.info(
        "AUDIT: Operator rejected intent [%s] from %s. Reason: %s",
        intent_id,
        request.client.host if request and request.client else "unknown",
        payload.reason if payload else "N/A",
    )

    pending_file.unlink()
    return {
        "intent_id": intent_id,
        "status": "rejected",
        "message": "Intent proposal purged from pending queue",
    }


@app.post("/api/panic")
def trigger_panic(
    request: Request = None,
    _auth: str = Depends(verify_admin_auth),
):
    """
    Emergency Global Panic Switch (Rule SAF-08).
    Immediately reverts all active cgroup modifications and persistently latches actuation disabled.
    """
    logger.warning(
        "AUDIT: 🚨 EMERGENCY PANIC SWITCH TRIGGERED via API from %s",
        request.client.host if request and request.client else "unknown",
    )

    res = send_actuator_ipc({"command": "panic"})
    if res.get("status") == "error":
        logger.critical("Actuator daemon reported error during emergency panic: %s", res.get("error"))
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Panic execution encountered errors: {res.get('error')}",
        )

    return {
        "status": "panic_engaged",
        "reverted_count": res.get("reverted_count", 0),
        "message": res.get("message"),
    }


@app.post("/api/panic/reset")
def reset_panic(
    request: Request = None,
    _auth: str = Depends(verify_admin_auth),
):
    """Clear emergency panic latch and re-enable actuation."""
    logger.warning(
        "AUDIT: Emergency panic latch reset via API from %s",
        request.client.host if request and request.client else "unknown",
    )

    res = send_actuator_ipc({"command": "reset_panic"})
    if res.get("status") == "error":
        raise HTTPException(
            status_code=status.HTTP_500_INTERNAL_SERVER_ERROR,
            detail=f"Panic reset failed: {res.get('error')}",
        )

    return {
        "status": "panic_cleared",
        "message": res.get("message"),
    }


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="127.0.0.1", port=8000)
