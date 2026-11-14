#!/usr/bin/env python3
"""
OS-FaultBench Workload: Relational Database Simulator.
Simulates high-concurrency transactional database workloads (like PostgreSQL pgbench).
Generates concurrent reads, updates, and WAL flushes.
"""

import argparse
import os
import sqlite3
import tempfile
import threading
import time


def db_worker(db_path: str, duration_sec: float, stop_event: threading.Event):
    conn = sqlite3.connect(db_path, timeout=10.0)
    cur = conn.cursor()
    cur.execute("PRAGMA journal_mode=WAL;")
    cur.execute("PRAGMA synchronous=NORMAL;")

    start = time.time()
    tx_count = 0

    while time.time() - start < duration_sec and not stop_event.is_set():
        try:
            # Point lookup + update
            cur.execute("BEGIN IMMEDIATE;")
            cur.execute("SELECT balance FROM accounts WHERE id = (abs(random()) % 1000) + 1;")
            cur.execute(
                "UPDATE accounts SET balance = balance + 1 WHERE id = (abs(random()) % 1000) + 1;"
            )
            cur.execute(
                "INSERT INTO history (account_id, delta, ts) VALUES ((abs(random()) % 1000) + 1, 1, ?);",
                (time.time(),),
            )
            conn.commit()
            tx_count += 1
            time.sleep(0.002)
        except Exception:
            try:
                conn.rollback()
            except Exception:
                pass
            time.sleep(0.01)

    conn.close()


class DatabaseWorkload:
    def __init__(self, clients: int = 4):
        self.clients = clients
        self._temp_dir = tempfile.mkdtemp(prefix="tsuki_db_")
        self._db_path = os.path.join(self._temp_dir, "benchmark.db")
        self._stop_event = threading.Event()
        self._threads = []

        self._init_db()

    def _init_db(self):
        conn = sqlite3.connect(self._db_path)
        cur = conn.cursor()
        cur.execute("CREATE TABLE accounts (id INTEGER PRIMARY KEY, balance INTEGER, pad TEXT);")
        cur.execute("CREATE TABLE history (id INTEGER PRIMARY KEY AUTOINCREMENT, account_id INTEGER, delta INTEGER, ts REAL);")
        # Populate 1,000 accounts
        cur.executemany(
            "INSERT INTO accounts (id, balance, pad) VALUES (?, 1000, ?);",
            [(i, "P" * 128) for i in range(1, 1001)],
        )
        conn.commit()
        conn.close()

    def start(self, duration_sec: float):
        self._stop_event.clear()
        self._threads = []
        for _ in range(self.clients):
            t = threading.Thread(
                target=db_worker,
                args=(self._db_path, duration_sec, self._stop_event),
                daemon=True,
            )
            t.start()
            self._threads.append(t)

    def stop(self):
        self._stop_event.set()
        for t in self._threads:
            t.join(timeout=2.0)

        # Cleanup
        try:
            for ext in ["", "-wal", "-shm"]:
                p = self._db_path + ext
                if os.path.exists(p):
                    os.remove(p)
            if os.path.exists(self._temp_dir):
                os.rmdir(self._temp_dir)
        except Exception:
            pass


def main():
    parser = argparse.ArgumentParser(description="Database Workload Simulator")
    parser.add_argument("--clients", type=int, default=4, help="Concurrent clients")
    parser.add_argument("--duration", type=int, default=5, help="Duration in seconds")
    args = parser.parse_args()

    print(f"[*] Starting Database Workload: {args.clients} clients for {args.duration}s...")
    wl = DatabaseWorkload(clients=args.clients)
    wl.start(args.duration)
    time.sleep(args.duration)
    wl.stop()
    print("[*] Database Workload finished.")


if __name__ == "__main__":
    main()
