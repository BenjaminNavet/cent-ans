"""API-equivalent cost of a Claude Code session (main transcript + subagents).

Usage: python3 tools/session_cost.py <session-id> [--project-dir DIR]
"""

import json
import sys
from pathlib import Path

# $/MTok: input, output, cache read. Cache writes: 1.25x input (5 min), 2x input (1 h).
PRICES = {
    "opus": (4.0, 20.0, 0.20),
    "sonnet": (2.0, 10.0, 0.20),
    "haiku": (0.10, 0.50, 0.01),
    "fable": (10.0, 50.0, 0.25),
}


def family(model_name: str) -> str | None:
    """Map a model id to its price family."""
    for name in PRICES:
        if name in model_name:
            return name
    return None


def entry_cost(model_name: str, usage: dict) -> float:
    """Dollar cost of one assistant message usage block."""
    fam = family(model_name)
    if fam is None:
        return 0.0
    price_in, price_out, price_read = PRICES[fam]
    creation = usage.get("cache_creation") or {}
    write_1h = creation.get("ephemeral_1h_input_tokens")
    write_5m = creation.get("ephemeral_5m_input_tokens")
    if write_1h is None and write_5m is None:
        write_1h, write_5m = usage.get("cache_creation_input_tokens", 0), 0
    tokens_cost = (
        usage.get("input_tokens", 0) * price_in
        + usage.get("output_tokens", 0) * price_out
        + usage.get("cache_read_input_tokens", 0) * price_read
        + (write_5m or 0) * price_in * 1.25
        + (write_1h or 0) * price_in * 2.0
    )
    return tokens_cost / 1e6


def transcript_cost(path: Path) -> tuple[float, str]:
    """Total cost of one jsonl transcript, deduplicated by message id."""
    seen: dict[str, float] = {}
    model_seen = "?"
    for line in path.open(errors="ignore"):
        try:
            record = json.loads(line)
        except json.JSONDecodeError:
            continue
        message = record.get("message")
        if not isinstance(message, dict) or "usage" not in message:
            continue
        model_name = message.get("model", "")
        model_seen = family(model_name) or model_seen
        key = message.get("id") or record.get("uuid")
        seen[key] = entry_cost(model_name, message["usage"])
    return sum(seen.values()), model_seen


def main() -> None:
    """Print per-transcript and total cost."""
    session_id = sys.argv[1]
    project_dir = Path.home() / ".claude/projects/-Users-jean-hubert-dev-game-project"
    if "--project-dir" in sys.argv:
        project_dir = Path(sys.argv[sys.argv.index("--project-dir") + 1])
    paths = [project_dir / f"{session_id}.jsonl"]
    paths += sorted((project_dir / session_id / "subagents").glob("*.jsonl"))
    total = 0.0
    for path in paths:
        if not path.exists():
            continue
        cost, fam = transcript_cost(path)
        total += cost
        if "-v" in sys.argv:
            print(f"{cost:8.2f} {fam:6} {path.stem}")
    print(f"TOTAL {total:.2f}")


if __name__ == "__main__":
    main()
