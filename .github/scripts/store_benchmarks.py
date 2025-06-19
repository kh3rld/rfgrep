"""
store_benchmarks.py

Parse Criterion benchmark results and store them in an SQLite database.
"""
import argparse
import json
import os
import sqlite3
from datetime import datetime

def parse_args():
    parser = argparse.ArgumentParser(description="Store benchmark results in SQLite DB.")
    parser.add_argument('--results-dir', required=True, help='Directory with Criterion JSON results')
    parser.add_argument('--db-file', required=True, help='SQLite database file')
    parser.add_argument('--commit-sha', required=True, help='Git commit SHA')
    parser.add_argument('--branch', required=True, help='Git branch name')
    return parser.parse_args()

def ensure_tables(conn):
    conn.execute('''CREATE TABLE IF NOT EXISTS benchmarks (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        commit_sha TEXT NOT NULL,
        branch TEXT NOT NULL,
        metric_category TEXT NOT NULL,
        metric_name TEXT NOT NULL,
        value REAL NOT NULL,
        timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
    )''')
    conn.execute('''CREATE TABLE IF NOT EXISTS runs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        commit_sha TEXT NOT NULL,
        branch TEXT NOT NULL,
        run_time DATETIME DEFAULT CURRENT_TIMESTAMP
    )''')
    conn.commit()

def store_benchmarks(conn, results_dir, commit_sha, branch):
    for root, _, files in os.walk(results_dir):
        for file in files:
            if file.endswith('.json'):
                path = os.path.join(root, file)
                with open(path) as f:
                    data = json.load(f)
                if isinstance(data, dict) and 'benchmarks' in data:
                    benchmarks = data['benchmarks']
                elif isinstance(data, list):
                    benchmarks = data
                else:
                    continue
                for bench in benchmarks:
                    if not isinstance(bench, dict):
                        continue
                    name = bench.get('name', 'unknown')
                    mean = bench.get('mean', {}).get('point_estimate')
                    if mean is not None:
                        conn.execute(
                            'INSERT INTO benchmarks (commit_sha, branch, metric_category, metric_name, value) VALUES (?, ?, ?, ?, ?)',
                            (commit_sha, branch, 'criterion', name, mean)
                        )
    conn.execute('INSERT INTO runs (commit_sha, branch) VALUES (?, ?)', (commit_sha, branch))
    conn.commit()

def main():
    args = parse_args()
    conn = sqlite3.connect(args.db_file)
    ensure_tables(conn)
    store_benchmarks(conn, args.results_dir, args.commit_sha, args.branch)
    print(f"Benchmarks stored for commit {args.commit_sha} on branch {args.branch}.")
    conn.close()

if __name__ == '__main__':
    main()
