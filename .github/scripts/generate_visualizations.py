"""
generate_visualizations.py

Generate interactive Plotly charts and Matplotlib trend lines from benchmark data.
"""
import argparse
import os
import sqlite3
import pandas as pd
import plotly.express as px
import matplotlib.pyplot as plt

def parse_args():
    parser = argparse.ArgumentParser(description="Generate benchmark visualizations.")
    parser.add_argument('--db-file', required=True, help='SQLite database file')
    parser.add_argument('--output-dir', required=True, help='Directory to save reports/plots')
    return parser.parse_args()

def main():
    args = parse_args()
    os.makedirs(args.output_dir, exist_ok=True)
    conn = sqlite3.connect(args.db_file)
    df = pd.read_sql_query("SELECT * FROM benchmarks", conn)
    if df.empty:
        print("No benchmark data found.")
        return
    # Plotly interactive chart (mean value by commit)
    fig = px.line(df, x='timestamp', y='value', color='metric_name',
                  title='Benchmark Trends', markers=True, hover_data=['commit_sha', 'branch'])
    fig.write_html(os.path.join(args.output_dir, 'benchmark_trends.html'))
    # Matplotlib trend line (for each metric)
    for metric in df['metric_name'].unique():
        metric_df = df[df['metric_name'] == metric]
        plt.figure(figsize=(10, 4))
        plt.plot(metric_df['timestamp'], metric_df['value'], marker='o')
        plt.title(f'Trend for {metric}')
        plt.xlabel('Timestamp')
        plt.ylabel('Value')
        plt.grid(True)
        plt.tight_layout()
        plt.savefig(os.path.join(args.output_dir, f'{metric}_trend.png'))
        plt.close()
    print(f"Visualizations saved to {args.output_dir}")
    conn.close()

if __name__ == '__main__':
    main()
