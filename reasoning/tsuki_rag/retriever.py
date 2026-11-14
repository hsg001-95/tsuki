"""
Tsuki Hybrid RAG Retriever.
Executes keyword matching (SQLite FTS5 BM25) and document section ranking.
Produces grounded citation items with unique chunk IDs for LLM agent reports.
"""

import os
import sqlite3
import sys
from typing import List

sys.path.insert(0, os.path.abspath("reasoning"))
from tsuki_agent.schemas import CitationItem


class HybridRetriever:
    def __init__(self, db_path: str = "reasoning/tsuki_rag/index.db"):
        self.db_path = db_path

    def search(self, query: str, k: int = 3) -> List[CitationItem]:
        if not os.path.exists(self.db_path):
            return []

        conn = sqlite3.connect(self.db_path)
        cur = conn.cursor()

        # Sanitize query for FTS5
        clean_words = [
            w for w in query.replace('"', "").replace("'", "").split() if len(w) > 2
        ]
        if not clean_words:
            conn.close()
            return []

        fts_query = " OR ".join(clean_words)

        results = []
        try:
            cur.execute(
                """
                SELECT chunk_id, source, section, title, snippet, rank
                FROM doc_chunks
                WHERE doc_chunks MATCH ?
                ORDER BY rank
                LIMIT ?;
                """,
                (fts_query, k),
            )
            rows = cur.fetchall()
            for r in rows:
                results.append(
                    CitationItem(
                        chunk_id=r[0],
                        source=r[1],
                        section=r[2],
                        title=r[3],
                        snippet=r[4][:250] + "..." if len(r[4]) > 250 else r[4],
                    )
                )
        except Exception:
            # Fallback simple LIKE matching
            cur.execute(
                """
                SELECT chunk_id, source, section, title, snippet
                FROM doc_chunks
                WHERE snippet LIKE ? OR title LIKE ?
                LIMIT ?;
                """,
                (f"%{clean_words[0]}%", f"%{clean_words[0]}%", k),
            )
            for r in cur.fetchall():
                results.append(
                    CitationItem(
                        chunk_id=r[0],
                        source=r[1],
                        section=r[2],
                        title=r[3],
                        snippet=r[4][:250] + "..." if len(r[4]) > 250 else r[4],
                    )
                )

        conn.close()
        return results
