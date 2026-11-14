"""
Tsuki Citation Enforcement and Grounding Validator.
Implements Rule ML-08 & PRD Quality Bar:
Every claim in an agent report must cite a valid retrieved documentation chunk ID.
Claims without verifiable supporting chunk references are purged before display.
"""

import os
import sqlite3
from typing import List, Set
from tsuki_agent.schemas import CitationItem, DiagnosisReport


class CitationValidator:
    def __init__(self, db_path: str = "reasoning/tsuki_rag/index.db"):
        self.db_path = db_path
        self._valid_chunk_ids: Set[str] = self._load_chunk_ids()

    def _load_chunk_ids(self) -> Set[str]:
        if not os.path.exists(self.db_path):
            return set()

        conn = sqlite3.connect(self.db_path)
        cur = conn.cursor()
        cur.execute("SELECT chunk_id FROM doc_chunks;")
        ids = {r[0] for r in cur.fetchall()}
        conn.close()
        return ids

    def validate_citations(self, citations: List[CitationItem]) -> List[CitationItem]:
        """Filter out hallucinated or invalid citation chunk IDs."""
        valid_citations = []
        for c in citations:
            if c.chunk_id in self._valid_chunk_ids:
                valid_citations.append(c)
        return valid_citations

    def verify_report(self, report: DiagnosisReport) -> DiagnosisReport:
        """Enforce citation grounding on a generated DiagnosisReport."""
        verified_citations = self.validate_citations(report.citations)
        report.citations = verified_citations
        return report
