"""
Tsuki Guard-to-Reasoning Snapshot Watcher & Transport (Rule SAF-01 / SAF-02).
Monitors guard plane evidence snapshots, triggers diagnosis agent,
and routes validated remediation intents to tsuki-actuator via Unix Domain Socket.
"""

import glob
import json
import logging
import os
import socket
import sys
import time
from pathlib import Path
from typing import Any, Dict, Optional, Set

sys.path.insert(0, os.path.abspath("reasoning"))
from tsuki_agent.agent import DiagnosisAgent
from tsuki_agent.schemas import DiagnosisReport, RemediationIntent
from tsuki_agent.tools import AgentTools

logger = logging.getLogger("tsuki.agent.watcher")


class SnapshotWatcher:
    def __init__(
        self,
        snapshot_dir: str = "/var/run/tsuki/snapshots",
        socket_path: str = "/var/run/tsuki/actuator.sock",
        pending_dir: str = "/var/run/tsuki/pending_intents",
        mode: Optional[str] = None,
        poll_interval_sec: float = 1.0,
    ):
        self.snapshot_dir = Path(os.environ.get("TSUKI_SNAPSHOT_DIR", snapshot_dir))
        self.socket_path = Path(os.environ.get("TSUKI_SOCKET_PATH", socket_path))
        self.pending_dir = Path(os.environ.get("TSUKI_PENDING_INTENTS_DIR", pending_dir))
        self.mode = mode or os.environ.get("TSUKI_MODE", "shadow")
        self.poll_interval_sec = poll_interval_sec

        tools = AgentTools(snapshot_dir=str(self.snapshot_dir))
        self.agent = DiagnosisAgent(tools=tools)
        self.processed_snapshots: Set[str] = set()

        # Ensure directories exist
        self.snapshot_dir.mkdir(parents=True, exist_ok=True)
        self.pending_dir.mkdir(parents=True, exist_ok=True)

    def send_actuator_command(self, payload: Dict[str, Any]) -> Dict[str, Any]:
        """Send command to tsuki-actuator over Unix Domain Socket."""
        if not self.socket_path.exists():
            raise FileNotFoundError(
                f"Actuator UDS socket not found at {self.socket_path}. Is tsuki-actuator running?"
            )

        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
            client.settimeout(5.0)
            client.connect(str(self.socket_path))
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
                raise RuntimeError("Empty response received from actuator daemon")
            return json.loads(raw)

    def process_snapshot(self, path: Path) -> Optional[DiagnosisReport]:
        """Load snapshot, run LLM/RAG diagnosis, and dispatch intents according to mode."""
        snap_id = path.stem
        if snap_id in self.processed_snapshots:
            return None

        logger.info("Processing new evidence snapshot: %s", path.name)
        try:
            with open(path, "r", encoding="utf-8") as f:
                data = json.load(f)
        except Exception as exc:
            logger.error("Failed to parse evidence snapshot %s: %s", path, exc, exc_info=True)
            self.processed_snapshots.add(snap_id)
            return None

        report = self.agent.diagnose_snapshot(data)
        logger.info(
            "Diagnosis completed for %s: fault_type=%s, candidate_intents=%d",
            snap_id,
            report.fault_type,
            len(report.candidate_intents),
        )

        for intent in report.candidate_intents:
            intent_dict = intent.model_dump()

            if self.mode == "autonomous":
                logger.info(
                    "Autonomous mode: dispatching intent [%s] to actuator daemon...",
                    intent.intent_id,
                )
                try:
                    res = self.send_actuator_command({"command": "apply", "intent": intent_dict})
                    logger.info("Actuator response for [%s]: %s", intent.intent_id, res)
                except Exception as exc:
                    logger.error(
                        "Actuation failed for intent [%s]: %s", intent.intent_id, exc, exc_info=True
                    )
            elif self.mode == "shadow":
                logger.info(
                    "Shadow mode: generated candidate intent [%s] (no actuation applied)",
                    intent.intent_id,
                )
            else:  # approval_required or custom
                logger.info(
                    "Approval required: staging intent [%s] for operator review",
                    intent.intent_id,
                )
                stage_file = self.pending_dir / f"{intent.intent_id}.json"
                with open(stage_file, "w", encoding="utf-8") as f:
                    json.dump(intent_dict, f, indent=2)

        self.processed_snapshots.add(snap_id)
        return report

    def poll_once(self) -> int:
        """Scan snapshot directory once and process any new files."""
        pattern = str(self.snapshot_dir / "*.json")
        files = sorted(glob.glob(pattern), key=os.path.getmtime)
        new_count = 0
        for fpath in files:
            p = Path(fpath)
            if p.stem not in self.processed_snapshots:
                self.process_snapshot(p)
                new_count += 1
        return new_count

    def run(self):
        """Continuous polling loop."""
        logger.info(
            "Starting Tsuki Snapshot Watcher on '%s' (mode=%s, socket='%s')",
            self.snapshot_dir,
            self.mode,
            self.socket_path,
        )
        while True:
            try:
                self.poll_once()
            except Exception as exc:
                logger.error("Watcher polling loop error: %s", exc, exc_info=True)
            time.sleep(self.poll_interval_sec)


if __name__ == "__main__":
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s [%(levelname)s] %(name)s: %(message)s",
    )
    watcher = SnapshotWatcher()
    watcher.run()
