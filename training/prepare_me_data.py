"""Convert MAP-Elites archive JSONL to chat-format training data.

Reads archive_*.jsonl files from data_me/ and produces train/valid/test
splits in JSONL chat format for MLX LoRA fine-tuning.

Usage:
    python prepare_me_data.py
    python prepare_me_data.py --data-dir data_me --split 0.9 0.05 0.05
"""

from __future__ import annotations

import argparse
import json
import random
from pathlib import Path

SYSTEM_PROMPT = (
    "You are a tilemap layout generator. Given a description of map properties, "
    "output a grid of tile names.\n"
    "Rules:\n"
    "- Each cell contains exactly one tile name from the tileset\n"
    "- Rows are separated by newlines\n"
    "- Tile names within a row are separated by spaces\n"
    "- The grid must match the specified dimensions\n"
    "- Output ONLY the grid, no explanation"
)


def archive_entry_to_chat(entry: dict) -> dict:
    """Convert a single archive entry to chat-format training data."""
    grid_str = "\n".join(" ".join(row) for row in entry["grid"])
    return {
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": entry["label"]},
            {"role": "assistant", "content": grid_str},
        ]
    }


def main():
    parser = argparse.ArgumentParser(description="Convert MAP-Elites archives to training data")
    parser.add_argument("--data-dir", type=str, default="data_me", help="Directory with archive JSONL files")
    parser.add_argument("--split", nargs=3, type=float, default=[0.9, 0.05, 0.05],
                        help="Train/valid/test split ratios")
    parser.add_argument("--seed", type=int, default=42, help="Random seed for split")
    args = parser.parse_args()

    data_dir = Path(__file__).resolve().parent / args.data_dir
    if not data_dir.exists():
        print(f"error: {data_dir} not found. Run map_elites.py first.")
        return

    # Load all archive entries
    entries = []
    for archive_file in sorted(data_dir.glob("archive_*.jsonl")):
        with open(archive_file) as f:
            for line in f:
                line = line.strip()
                if line:
                    entries.append(json.loads(line))
        print(f"loaded {archive_file.name}")

    if not entries:
        print("error: no archive entries found")
        return

    print(f"total entries: {len(entries)}")

    # Convert to chat format
    chat_data = [archive_entry_to_chat(e) for e in entries]

    # Shuffle and split
    random.seed(args.seed)
    random.shuffle(chat_data)

    train_ratio, valid_ratio, _ = args.split
    n = len(chat_data)
    n_train = int(n * train_ratio)
    n_valid = int(n * valid_ratio)

    train = chat_data[:n_train]
    valid = chat_data[n_train : n_train + n_valid]
    test = chat_data[n_train + n_valid :]

    print(f"split: {len(train)} train, {len(valid)} valid, {len(test)} test")

    # Write JSONL files
    for name, data in [("train", train), ("valid", valid), ("test", test)]:
        out_path = data_dir / f"{name}.jsonl"
        with open(out_path, "w") as f:
            for item in data:
                f.write(json.dumps(item) + "\n")
        print(f"wrote {out_path}")

    # Stats
    avg_tokens_est = sum(
        len(d["messages"][2]["content"].split()) for d in chat_data
    ) / len(chat_data)
    print(f"avg grid tokens (whitespace-split): {avg_tokens_est:.0f}")


if __name__ == "__main__":
    main()
