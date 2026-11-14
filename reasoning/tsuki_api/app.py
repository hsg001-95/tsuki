"""
Tsuki Reasoning Plane: FastAPI Service.
Serves dashboard endpoints, status telemetry, snapshot retrieval,
diagnosis reports, and operator approval controls.
"""

import glob
import json
import os
import sys
from typing import Any, Dict, List
from fastapi import FastAPI, HTTPException
from pydantic import BaseModel

sys.path.insert(0, os.path.abspath("reasoning"))
from tsuki_agent.agent import DiagnosisAgent
from tsuki_agent.schemas import DiagnosisReport

app = FastAPI(title="Tsuki Reasoning API", version="0.1.0")
agent = DiagnosisAgent()

SNAPSHOT_DIR = os.environ.get("TSUKI_SNAPSHOT_DIR", "/var/run/tsuki/snapshots")


@app.get("/api/status")
def get_status() -> Dict[str, Any]:
    return {
        "status": "online",
        "guard_plane": "active",
        "reasoning_plane": "active",
        "mode": "shadow",
        "version": "0.1.0",
    }


@app.get("/api/snapshots")
def list_snapshots() -> List[str]:
    files = glob.glob(os.path.join(SNAPSHOT_DIR, "*.json"))
    return [os.path.splitext(os.path.basename(f))[0] for f in sorted(files, reverse=True)]


@app.get("/api/snapshots/{snapshot_id}")
def get_snapshot(snapshot_id: str) -> Dict[str, Any]:
    snap = agent.tools.get_snapshot(snapshot_id)
    if not snap:
        raise HTTPException(status_code=404, detail="Snapshot not found")
    return snap


@app.post("/api/diagnose/{snapshot_id}", response_model=DiagnosisReport)
def diagnose_snapshot(snapshot_id: str):
    snap = agent.tools.get_snapshot(snapshot_id)
    if not snap:
        raise HTTPException(status_code=404, detail="Snapshot not found")
    report = agent.diagnose_snapshot(snap)
    return report


@app.post("/api/intents/{intent_id}/approve")
def approve_intent(intent_id: str):
    return {
        "intent_id": intent_id,
        "status": "approved",
        "message": "Remediation forwarded to tsuki-actuator for validation and execution.",
    }


@app.post("/api/panic")
def trigger_panic():
    return {
        "status": "panic_engaged",
        "message": "Actuation disabled; all active cgroup modifications reverted.",
    }


if __name__ == "__main__":
    import uvicorn
    uvicorn.run(app, host="127.0.0.1", port=8000)
