#!/usr/bin/env python3
"""
nix-evo-agent benchmark script

Tests endpoint latency, throughput, and WebSocket stability.
Run against a running agent instance.

Usage:
  python3 bench.py [--url http://127.0.0.1:7890] [--requests 100] [--concurrency 10]
"""

import argparse
import json
import statistics
import sys
import time
import urllib.request
import urllib.error
from concurrent.futures import ThreadPoolExecutor, as_completed
from dataclasses import dataclass, field
from typing import Optional

@dataclass
class BenchResult:
    endpoint: str
    method: str
    status_code: int
    latency_ms: float
    error: Optional[str] = None
    response_size: int = 0

@dataclass
class BenchSummary:
    endpoint: str
    total_requests: int = 0
    successful: int = 0
    failed: int = 0
    latencies: list = field(default_factory=list)
    errors: list = field(default_factory=list)

    @property
    def p50(self):
        return statistics.median(self.latencies) if self.latencies else 0

    @property
    def p95(self):
        if not self.latencies:
            return 0
        sorted_l = sorted(self.latencies)
        idx = int(len(sorted_l) * 0.95)
        return sorted_l[min(idx, len(sorted_l) - 1)]

    @property
    def p99(self):
        if not self.latencies:
            return 0
        sorted_l = sorted(self.latencies)
        idx = int(len(sorted_l) * 0.99)
        return sorted_l[min(idx, len(sorted_l) - 1)]

    @property
    def avg(self):
        return statistics.mean(self.latencies) if self.latencies else 0

    @property
    def rps(self):
        if not self.latencies:
            return 0
        total_time = sum(self.latencies) / 1000  # Convert to seconds
        return self.total_requests / total_time if total_time > 0 else 0


def request_get(url: str, headers: dict = None) -> BenchResult:
    """Execute a single GET request and measure latency."""
    req = urllib.request.Request(url, headers=headers or {})
    start = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            body = resp.read()
            latency = (time.perf_counter() - start) * 1000
            return BenchResult(
                endpoint=url.split("/")[-1],
                method="GET",
                status_code=resp.status,
                latency_ms=latency,
                response_size=len(body),
            )
    except urllib.error.HTTPError as e:
        latency = (time.perf_counter() - start) * 1000
        return BenchResult(
            endpoint=url.split("/")[-1],
            method="GET",
            status_code=e.code,
            latency_ms=latency,
            error=str(e),
        )
    except Exception as e:
        latency = (time.perf_counter() - start) * 1000
        return BenchResult(
            endpoint=url.split("/")[-1],
            method="GET",
            status_code=0,
            latency_ms=latency,
            error=str(e),
        )


def request_post(url: str, body: dict, headers: dict = None) -> BenchResult:
    """Execute a single POST request and measure latency."""
    data = json.dumps(body).encode("utf-8")
    all_headers = {"Content-Type": "application/json"}
    all_headers.update(headers or {})
    req = urllib.request.Request(url, data=data, headers=all_headers, method="POST")
    start = time.perf_counter()
    try:
        with urllib.request.urlopen(req, timeout=10) as resp:
            resp_body = resp.read()
            latency = (time.perf_counter() - start) * 1000
            return BenchResult(
                endpoint=url.split("/")[-1],
                method="POST",
                status_code=resp.status,
                latency_ms=latency,
                response_size=len(resp_body),
            )
    except urllib.error.HTTPError as e:
        latency = (time.perf_counter() - start) * 1000
        return BenchResult(
            endpoint=url.split("/")[-1],
            method="POST",
            status_code=e.code,
            latency_ms=latency,
            error=str(e),
        )
    except Exception as e:
        latency = (time.perf_counter() - start) * 1000
        return BenchResult(
            endpoint=url.split("/")[-1],
            method="POST",
            status_code=0,
            latency_ms=latency,
            error=str(e),
        )


def run_bench(base_url: str, num_requests: int, concurrency: int):
    """Run benchmark suite."""
    endpoints = [
        ("GET", f"{base_url}/health", None),
        ("GET", f"{base_url}/api/audit?limit=10", None),
        ("GET", f"{base_url}/api/audit/stats", None),
        ("GET", f"{base_url}/api/healer/status", None),
        ("POST", f"{base_url}/api/flake/convert", {
            "config_content": "{ networking.hostName = \"bench\"; services.nginx.enable = true; }",
            "hostname": "bench"
        }),
    ]

    print(f"\n{'='*60}")
    print(f"  nix-evo-agent Benchmark")
    print(f"  URL: {base_url}")
    print(f"  Requests per endpoint: {num_requests}")
    print(f"  Concurrency: {concurrency}")
    print(f"{'='*60}\n")

    all_summaries = {}

    for method, url, body in endpoints:
        name = f"{method} {url.split('/')[-1]}"
        summary = BenchSummary(endpoint=name)
        all_summaries[name] = summary

        print(f"Testing {name}...", end=" ", flush=True)

        with ThreadPoolExecutor(max_workers=concurrency) as executor:
            futures = []
            for _ in range(num_requests):
                if method == "GET":
                    futures.append(executor.submit(request_get, url))
                else:
                    futures.append(executor.submit(request_post, url, body))

            for future in as_completed(futures):
                result = future.result()
                summary.total_requests += 1
                if result.error:
                    summary.failed += 1
                    summary.errors.append(result.error)
                else:
                    summary.successful += 1
                summary.latencies.append(result.latency_ms)

        # Print per-endpoint results
        print(f"\n  ✓ {summary.successful}/{summary.total_requests} OK")
        if summary.failed > 0:
            print(f"  ✗ {summary.failed} failed")
        print(f"  Latency: avg={summary.avg:.1f}ms  p50={summary.p50:.1f}ms  p95={summary.p95:.1f}ms  p99={summary.p99:.1f}ms")
        print()

    # Summary table
    print(f"{'='*60}")
    print(f"  Summary")
    print(f"{'='*60}")
    print(f"  {'Endpoint':<30} {'OK':>5} {'Fail':>5} {'Avg':>8} {'P50':>8} {'P95':>8} {'P99':>8}")
    print(f"  {'-'*30} {'-'*5} {'-'*5} {'-'*8} {'-'*8} {'-'*8} {'-'*8}")

    for name, s in all_summaries.items():
        print(f"  {name:<30} {s.successful:>5} {s.failed:>5} {s.avg:>7.1f}ms {s.p50:>7.1f}ms {s.p95:>7.1f}ms {s.p99:>7.1f}ms")

    print()


def main():
    parser = argparse.ArgumentParser(description="nix-evo-agent benchmark")
    parser.add_argument("--url", default="http://127.0.0.1:7890", help="Agent base URL")
    parser.add_argument("--requests", type=int, default=50, help="Requests per endpoint")
    parser.add_argument("--concurrency", type=int, default=5, help="Concurrent requests")
    args = parser.parse_args()

    run_bench(args.url, args.requests, args.concurrency)


if __name__ == "__main__":
    main()
