"""Prepare training data for MLX LoRA fine-tuning.

Converts the corpus training pairs (description, pax_grid) into
chat-format JSONL that mlx-lm expects for fine-tuning.
"""

import json
import os
import random

CORPUS_DIR = os.path.join(os.path.dirname(__file__), "..", "tool", "corpus")
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "data")

SYSTEM_PROMPT = """You are a pixel art tile generator. Given a description, output a PAX-format character grid.
Rules:
- Use only the symbols from the palette provided
- Each row must be exactly the specified width
- Total rows must equal the specified height
- '.' means transparent/void
- Output ONLY the grid, no explanation"""

def load_all_pairs():
    pairs = []
    for fname in os.listdir(CORPUS_DIR):
        if fname.endswith("_training.json"):
            path = os.path.join(CORPUS_DIR, fname)
            with open(path) as f:
                data = json.load(f)
                for desc, grid in data:
                    if grid.strip():  # skip empty grids
                        pairs.append((desc, grid))
    return pairs


def to_chat_format(desc, grid):
    """Convert a (description, grid) pair to chat format for mlx-lm."""
    return {
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": desc},
            {"role": "assistant", "content": grid},
        ]
    }


def to_completion_format(desc, grid):
    """Convert to simple completion format (prompt + completion)."""
    prompt = f"Generate a PAX pixel art grid:\n{desc}\n\nGrid:\n"
    return {"text": prompt + grid}


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    pairs = load_all_pairs()
    print(f"Loaded {len(pairs)} training pairs from {CORPUS_DIR}")

    # Shuffle deterministically
    random.seed(42)
    random.shuffle(pairs)

    # Split: 90% train, 5% valid, 5% test
    n = len(pairs)
    train_end = int(n * 0.9)
    valid_end = int(n * 0.95)

    train = pairs[:train_end]
    valid = pairs[train_end:valid_end]
    test = pairs[valid_end:]

    print(f"Split: {len(train)} train, {len(valid)} valid, {len(test)} test")

    # Write JSONL files in chat format (for mlx-lm)
    for split_name, split_data in [("train", train), ("valid", valid), ("test", test)]:
        path = os.path.join(OUTPUT_DIR, f"{split_name}.jsonl")
        with open(path, "w") as f:
            for desc, grid in split_data:
                entry = to_chat_format(desc, grid)
                f.write(json.dumps(entry) + "\n")
        print(f"Wrote {path} ({len(split_data)} entries)")

    # Also write completion format as fallback
    for split_name, split_data in [("train", train), ("valid", valid), ("test", test)]:
        path = os.path.join(OUTPUT_DIR, f"{split_name}_completion.jsonl")
        with open(path, "w") as f:
            for desc, grid in split_data:
                entry = to_completion_format(desc, grid)
                f.write(json.dumps(entry) + "\n")
        print(f"Wrote {path} ({len(split_data)} entries)")

    # Stats
    avg_grid_len = sum(len(grid) for _, grid in pairs) / len(pairs)
    print(f"\nAverage grid length: {avg_grid_len:.0f} chars")
    print(f"Total tokens (estimate): ~{int(avg_grid_len * len(pairs) / 4)} tokens")


if __name__ == "__main__":
    main()
