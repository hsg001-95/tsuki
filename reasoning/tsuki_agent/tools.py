"""
Tsuki Read-Only Agent Tools.
Enforces Rule SAF-01: Zero write authority, no shell execution,
no direct cgroup writes, no network egress.
Tools return Pydantic-validated structured data only.
"""

import glob
import json
import logging
import os
from typing import Any, Dict, List, Optional
from pydantic import BaseModel, Field

from tsuki_agent.schemas import CitationItem
from tsuki_rag.retriever import HybridRetriever

logger = logging.getLogger("tsuki.agent.tools")


class CgroupSummary(BaseModel):
    cgroup_path: str
    memory_current_bytes: int
    cpu_usage_usec: int


class AgentTools:
    def __init__(
        self,
        snapshot_dir: str = "/var/run/tsuki/snapshots",
        index_db: str = "reasoning/tsuki_rag/index.db",
        config_path: str = "tsuki.toml.example",
    ):
        self.snapshot_dir = snapshot_dir
        self.retriever = HybridRetriever(db_path=index_db)
        self.config_path = config_path

    def get_snapshot(self, snapshot_id: str) -> Optional[Dict[str, Any]]:
        """Retrieve an immutable evidence snapshot from the guard plane ring buffer."""
        # Try exact filename or find in snapshot_dir
        path = os.path.join(self.snapshot_dir, f"{snapshot_id}.json")
        if not os.path.exists(path):
            candidates = glob.glob(os.path.join(self.snapshot_dir, f"*{snapshot_id}*.json"))
            if candidates:
                path = candidates[0]
            else:
                return None

        try:
            with open(path, "r", encoding="utf-8") as f:
                return json.load(f)
        except Exception as exc:
            logger.error("Failed to load evidence snapshot from %s: %s", path, exc, exc_info=True)
            raise RuntimeError(f"Snapshot loading failed for '{path}': {exc}") from exc

    def top_cgroups(self, snapshot_id: str, n: int = 5) -> List[CgroupSummary]:
        """Rank cgroups by memory utilization from an evidence snapshot."""
        snap = self.get_snapshot(snapshot_id)
        if not snap or not snap.get("samples"):
            return []

        latest = snap["samples"][-1]
        cgroups = latest.get("cgroups", [])

        # Sort descending by memory
        sorted_cgroups = sorted(
            cgroups, key=lambda c: c.get("memory_current_bytes", 0), reverse=True
        )

        return [
            CgroupSummary(
                cgroup_path=c["cgroup_path"],
                memory_current_bytes=c.get("memory_current_bytes", 0),
                cpu_usage_usec=c.get("cpu_usage_usec", 0),
            )
            for c in sorted_cgroups[:n]
        ]

    def search_docs(self, query: str, k: int = 3) -> List[CitationItem]:
        """Query kernel, cgroup, and systemd documentation index."""
        return self.retriever.search(query, k=k)

    def protected_list(self) -> Dict[str, List[str]]:
        """Retrieve the configured protected targets that can never be modified (SAF-05)."""
        protected_cgroups = [
            "/system.slice/sshd.service",
            "/system.slice/dbus.service",
            "/system.slice/systemd-journald.service",
            "/user.slice",
        ]
        protected_processes = [
            "systemd",
            "sshd",
            "Xorg",
            "wayland",
            "gnome-shell",
            "dbus-daemon",
            "tsuki-guard",
            "tsuki-actuator",
            "tsuki-rollbackd",
        ]

        if os.path.exists(self.config_path):
            try:
                import tomllib
                with open(self.config_path, "rb") as f:
                    cfg = tomllib.load(f)
                    p = cfg.get("protected", {})
                    if "cgroups" in p:
                        protected_cgroups = p["cgroups"]
                    if "process_names" in p:
                        protected_processes = p["process_names"]
            except Exception as exc:
                logger.error("Failed to parse protected config from %s: %s", self.config_path, exc, exc_info=True)
                raise RuntimeError(
                    f"Configuration parsing error in protected-list at '{self.config_path}': {exc}"
                ) from exc

        return {
            "cgroups": protected_cgroups,
            "processes": protected_processes,
        }
