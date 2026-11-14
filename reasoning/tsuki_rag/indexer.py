#!/usr/bin/env python3
"""
Tsuki Documentation Indexer.
Parses curated Linux kernel, cgroup v2, PSI, and systemd documentation.
Builds an embedded SQLite FTS5 search index (BM25) with structured chunk metadata
(chunk_id, source, section, title, snippet, license).
Zero external service dependencies (Rule DEP-01).
"""

import glob
import os
import sqlite3
from typing import Dict, List


def parse_markdown_corpus(corpus_dir: str) -> List[Dict[str, str]]:
    chunks = []
    md_files = glob.glob(os.path.join(corpus_dir, "*.md"))

    for filepath in md_files:
        with open(filepath, "r", encoding="utf-8") as f:
            content = f.read()

        lines = content.split("\n")
        doc_source = "kernel-docs"
        doc_license = "GPL-2.0"

        # Read top metadata
        for line in lines[:10]:
            if line.startswith("Source:"):
                doc_source = line.split("Source:", 1)[1].strip()
            elif line.startswith("License:"):
                doc_license = line.split("License:", 1)[1].strip()

        # Split on section headers (## )
        current_chunk = None
        for line in lines:
            if line.startswith("## "):
                if current_chunk and current_chunk.get("snippet"):
                    chunks.append(current_chunk)

                title = line[3:].strip()
                current_chunk = {
                    "chunk_id": f"doc_{len(chunks)}",
                    "title": title,
                    "source": doc_source,
                    "section": title,
                    "snippet": "",
                    "license": doc_license,
                }
            elif current_chunk:
                if line.startswith("chunk_id:"):
                    current_chunk["chunk_id"] = line.split("chunk_id:", 1)[1].strip()
                elif line.startswith("Section:"):
                    current_chunk["section"] = line.split("Section:", 1)[1].strip()
                else:
                    if line.strip():
                        current_chunk["snippet"] += line + " "

        if current_chunk and current_chunk.get("snippet"):
            current_chunk["snippet"] = current_chunk["snippet"].strip()
            chunks.append(current_chunk)

    return chunks


def build_sqlite_fts5_index(
    chunks: List[Dict[str, str]], db_path: str = "reasoning/tsuki_rag/index.db"
):
    os.makedirs(os.path.dirname(db_path), exist_ok=True)
    if os.path.exists(db_path):
        os.remove(db_path)

    conn = sqlite3.connect(db_path)
    cur = conn.cursor()

    # Create FTS5 table
    cur.execute(
        """
        CREATE VIRTUAL TABLE doc_chunks USING fts5(
            chunk_id UNINDEXED,
            title,
            source,
            section,
            snippet,
            tokenize='porter unicode61'
        );
        """
    )

    for c in chunks:
        cur.execute(
            """
            INSERT INTO doc_chunks (chunk_id, title, source, section, snippet)
            VALUES (?, ?, ?, ?, ?);
            """,
            (c["chunk_id"], c["title"], c["source"], c["section"], c["snippet"]),
        )

    conn.commit()
    conn.close()
    print(f"[✅] Built SQLite FTS5 index with {len(chunks)} chunks at: {db_path}")


def main():
    corpus_dir = "reasoning/tsuki_rag/corpus"
    db_path = "reasoning/tsuki_rag/index.db"
    chunks = parse_markdown_corpus(corpus_dir)
    build_sqlite_fts5_index(chunks, db_path)


if __name__ == "__main__":
    main()
