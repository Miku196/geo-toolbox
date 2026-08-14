#!/usr/bin/env python3
"""End-to-end test script for geo-toolbox-agent.

Simulates frontend interaction by sending a natural-language query to the
`/agent` endpoint (localhost:3000), parsing the JSON response, and printing
tool_call details.

Supports:
  - Real HTTP call against a running agent
  - --mock flag to demo without a running agent
  - Graceful connection error handling
  - [FALLBACK] and [ERROR] markers for UI-consumable output

Usage:
  python tests/e2e_test.py --query "分析德兴铜矿NDVI"
  python tests/e2e_test.py --mock               # demo without agent running
  python tests/e2e_test.py --query "..." --json # emit raw JSON response
  python tests/e2e_test.py --map                # render a folium map if available
"""

from __future__ import annotations

import argparse
import json
import os
import sys
import textwrap
from typing import Any, Dict, List, Optional, Tuple

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

DEFAULT_BASE_URL: str = "http://localhost:3000"
AGENT_ENDPOINT: str = "/agent"
REQUEST_TIMEOUT: int = 30  # seconds

# Demo data used when --mock is passed
MOCK_RESPONSE: Dict[str, Any] = {
    "fallback": False,
    "provider": "openai",
    "model": "gpt-4o",
    "tool_calls": [
        {
            "tool": "ndvi_analysis",
            "params": {
                "aoi": "德兴铜矿",
                "start_date": "2024-06-01",
                "end_date": "2024-08-31",
                "sensor": "sentinel-2",
                "clip": True,
                "output_format": "geotiff",
            },
        },
        {
            "tool": "map_view",
            "params": {"center": [28.96, 117.58], "zoom": 13, "basemap": "satellite"},
        },
    ],
    "usage": {"prompt_tokens": 245, "completion_tokens": 89, "total_tokens": 334},
}

MOCK_FALLBACK_RESPONSE: Dict[str, Any] = {
    "fallback": True,
    "provider": "fallback",
    "model": "keyword-router",
    "tool_calls": [
        {
            "tool": "ndvi_analysis",
            "params": {"aoi": "德兴铜矿"},
        }
    ],
    "usage": None,
}


# ---------------------------------------------------------------------------
# HTTP helpers
# ---------------------------------------------------------------------------

def _try_import_requests() -> Any:
    """Import requests (stdlib-friendly courtesy)."""
    try:
        import requests  # type: ignore[import-untyped]
    except ImportError:
        print("[ERROR] 'requests' package is required for HTTP calls.", file=sys.stderr)
        print("        Install with: pip install requests", file=sys.stderr)
        sys.exit(3)
    return requests


# ---------------------------------------------------------------------------
# Core logic
# ---------------------------------------------------------------------------

def send_query(query: str, base_url: str = DEFAULT_BASE_URL) -> Optional[Dict[str, Any]]:
    """POST *query* to /agent and return the parsed JSON dict, or None on failure."""
    requests = _try_import_requests()
    url = base_url.rstrip("/") + AGENT_ENDPOINT
    payload = {"query": query, "force_fallback": False}

    print(f"[INFO] POST {url}")
    print(f"[INFO] Query: {query}")

    try:
        resp = requests.post(
            url,
            json=payload,
            timeout=REQUEST_TIMEOUT,
            headers={"Content-Type": "application/json"},
        )
    except requests.exceptions.ConnectionError:
        print("[ERROR] Connection refused — is the geo-toolbox-agent running?")
        print(f"[ERROR] Expected address: {url}")
        return None
    except requests.exceptions.Timeout:
        print(f"[ERROR] Request timed out after {REQUEST_TIMEOUT}s")
        return None
    except requests.exceptions.RequestException as exc:
        print(f"[ERROR] HTTP error: {exc}")
        return None

    if resp.status_code == 404:
        # The agent's "no match" response
        try:
            body = resp.json()
            msg = body.get("error", "No matching tool found.")
        except ValueError:
            msg = resp.text.strip() or "Not found"
        print(f"[ERROR] 404 — {msg}")
        return None

    if not resp.ok:
        print(f"[ERROR] Unexpected status {resp.status_code}: {resp.text[:500]}")
        return None

    try:
        data: Dict[str, Any] = resp.json()
    except ValueError:
        print(f"[ERROR] Response is not valid JSON: {resp.text[:500]}")
        return None

    return data


# ---------------------------------------------------------------------------
# Response printing
# ---------------------------------------------------------------------------

def print_tool_calls(
    data: Dict[str, Any],
    *,
    show_raw: bool = False,
    indent: int = 2,
) -> None:
    """Pretty-print tool_calls from an AgentResponse dict.

    Handles:
      - fallback=true  → [FALLBACK] banner
      - normal calls   → [TOOL] name + params
      - empty calls    → warning
    """
    if show_raw:
        print("-" * 60)
        print(json.dumps(data, ensure_ascii=False, indent=indent))
        print("-" * 60)
        print()

    is_fallback = bool(data.get("fallback"))
    provider = data.get("provider", "?")
    model = data.get("model", "?")
    tool_calls: List[Dict[str, Any]] = data.get("tool_calls", [])
    usage = data.get("usage")

    # --- fallback banner ---
    if is_fallback:
        print("[FALLBACK] LLM unavailable — keyword router responded")

    # --- provider info ---
    print(f"[INFO] provider={provider}  model={model}")

    # --- usage ---
    if usage:
        print(
            f"[INFO] tokens: prompt={usage.get('prompt_tokens','?')} "
            f"completion={usage.get('completion_tokens','?')} "
            f"total={usage.get('total_tokens','?')}"
        )
    print()

    # --- tool calls ---
    if not tool_calls:
        print("[WARN] No tool_calls in response.")
        return

    for i, tc in enumerate(tool_calls, start=1):
        tool = tc.get("tool", "?")
        params = tc.get("params", {})
        print(f"[TOOL] {tool}")
        for k, v in params.items():
            print(f"        {k} = {v!r}")
        print()

    print(f"[INFO] {len(tool_calls)} tool call(s) received.")


# ---------------------------------------------------------------------------
# Optional folium map
# ---------------------------------------------------------------------------

def render_map(data: Dict[str, Any], output_path: str = "e2e_map.html") -> bool:
    """Create a simple folium map centred on the first tool-call coordinate.

    Returns True on success, False if folium is unavailable or no coords found.
    """
    try:
        import folium  # type: ignore[import-untyped]
    except ImportError:
        print("[INFO] folium not installed — skipping map render.")
        print("       Install with: pip install folium")
        return False

    # Attempt to locate a coordinate in any tool_call param
    centre: Optional[Tuple[float, float]] = None
    for tc in data.get("tool_calls", []):
        params: Dict[str, Any] = tc.get("params", {})
        if "center" in params:
            centre = tuple(params["center"][:2])  # type: ignore[assignment]
            break
        if "aoi" in params:
            # approximate centroid for known locations
            aoi = str(params["aoi"]).lower()
            known: Dict[str, Tuple[float, float]] = {
                "德兴铜矿": (28.96, 117.58),
                "dexing": (28.96, 117.58),
                "德兴": (28.96, 117.58),
            }
            if aoi in known:
                centre = known[aoi]
                break

    if centre is None:
        print("[INFO] No coordinates found for map — using default (28.96, 117.58)")
        centre = (28.96, 117.58)

    m = folium.Map(location=list(centre), zoom_start=12)
    folium.Marker(list(centre), popup="Query AOI").add_to(m)

    m.save(output_path)
    print(f"[INFO] Map saved to {os.path.abspath(output_path)}")
    return True


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def build_parser() -> argparse.ArgumentParser:
    p = argparse.ArgumentParser(
        description="End-to-end test for geo-toolbox-agent /agent endpoint",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=textwrap.dedent(
            """\
            Examples:
              python tests/e2e_test.py --query "分析德兴铜矿NDVI"
              python tests/e2e_test.py --mock
              python tests/e2e_test.py --mock --json
              python tests/e2e_test.py --query "..." --map
            """
        ),
    )
    p.add_argument("--query", "-q", default=None, help="Natural language query to send.")
    p.add_argument(
        "--base-url",
        default=DEFAULT_BASE_URL,
        help=f"Base URL of the agent server (default: {DEFAULT_BASE_URL})",
    )
    p.add_argument(
        "--mock",
        action="store_true",
        help="Skip HTTP call; use built-in mock response for demo.",
    )
    p.add_argument(
        "--json",
        action="store_true",
        dest="show_raw",
        help="Also print the raw JSON response.",
    )
    p.add_argument(
        "--map",
        action="store_true",
        dest="render_map",
        help="Render a folium map centred on the query AOI.",
    )
    return p


def main() -> None:
    parser = build_parser()
    args = parser.parse_args()

    # ------------------------------------------------------------------
    # 1. Obtain response data
    # ------------------------------------------------------------------
    if args.mock:
        query = args.query or "分析德兴铜矿NDVI"
        print(f"[INFO] --mock mode  (query: {query})")

        # If query mentions "fallback" we use the fallback mock
        if "fallback" in query.lower():
            data: Optional[Dict[str, Any]] = MOCK_FALLBACK_RESPONSE
        else:
            data = dict(MOCK_RESPONSE)
    else:
        if not args.query:
            parser.error("--query is required unless --mock is used.")
        data = send_query(args.query, base_url=args.base_url)

    # ------------------------------------------------------------------
    # 2. Handle missing / error
    # ------------------------------------------------------------------
    if data is None:
        print()
        print("[INFO] Response was empty or unreachable.")
        print("[INFO] Tip: re-run with --mock to see example behaviour.")
        sys.exit(2)

    # ------------------------------------------------------------------
    # 3. Print
    # ------------------------------------------------------------------
    print_tool_calls(data, show_raw=args.show_raw)

    # ------------------------------------------------------------------
    # 4. Optional map
    # ------------------------------------------------------------------
    if args.render_map:
        print()
        render_map(data)

    print("[DONE]")


if __name__ == "__main__":
    main()
