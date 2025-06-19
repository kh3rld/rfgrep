"""
check_regressions.py

Compare current benchmark results to a baseline and exit nonzero if regression detected.
"""
import argparse
import sqlite3
import sys
import pandas as pd

def parse_args():
    parser = argparse.ArgumentParser(description="Check for benchmark regressions.")
    parser.add_argument('--db-file', required=True, help='SQLite database file')
    parser.add_argument('--current-commit', required=True, help='Current commit SHA')
    parser.add_argument('--baseline-branch', required=True, help='Baseline branch name')
    parser.add_argument('--threshold-percentage', type=float, default=5.0, help='Regression threshold (%)')
    return parser.parse_args()

def main():
    args = parse_args()
    conn = sqlite3.connect(args.db_file)
    df = pd.read_sql_query("SELECT * FROM benchmarks", conn)
    if df.empty:
        print("No benchmark data found.")
        sys.exit(0)
    # Get current and baseline results
    current = df[df['commit_sha'] == args.current_commit]
    baseline = df[(df['branch'] == args.baseline_branch)]
    if current.empty or baseline.empty:
        print("No data for current commit or baseline branch.")
        sys.exit(0)
    # Compare each metric
    regression_found = False
    for metric in current['metric_name'].unique():
        cur_val = current[current['metric_name'] == metric]['value'].mean()
        base_val = baseline[baseline['metric_name'] == metric]['value'].mean()
        if cur_val > base_val * (1 + args.threshold_percentage / 100):
            print(f"Regression detected in {metric}: {cur_val:.3f} > {base_val:.3f} (+{args.threshold_percentage}% threshold)")
            regression_found = True
    conn.close()
    if regression_found:
        sys.exit(1)
    print("No regressions detected.")
    sys.exit(0)

if __name__ == '__main__':
    main()
