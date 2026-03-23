"""LoRA fine-tuning script for PAX tile generation using MLX on Apple Silicon.

Uses mlx-lm to fine-tune a small language model on PAX grid data.
The model learns the symbolic vocabulary and spatial patterns of pixel art tiles.

Usage:
  cd training
  source .venv/bin/activate
  python train.py
"""

import subprocess
import sys
import os

# ── Configuration ──────────────────────────────────────

# Model: Qwen2.5-3B-Instruct — small enough for 24GB M4 Pro, good at structured output
MODEL = "mlx-community/Qwen2.5-3B-Instruct-4bit"

DATA_DIR = os.path.join(os.path.dirname(__file__), "data")
OUTPUT_DIR = os.path.join(os.path.dirname(__file__), "adapters", "pixl-lora")

# LoRA hyperparameters
LORA_RANK = 8
LORA_LAYERS = 16       # number of layers to apply LoRA to
BATCH_SIZE = 1          # keep small for 24GB
LEARNING_RATE = 1e-5
EPOCHS = 3              # 3 passes per SELF-REFINE research
MAX_SEQ_LENGTH = 512    # PAX grids are ~271 chars avg
STEPS_PER_EVAL = 100
SAVE_EVERY = 200


def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)

    train_file = os.path.join(DATA_DIR, "train.jsonl")
    valid_file = os.path.join(DATA_DIR, "valid.jsonl")
    test_file = os.path.join(DATA_DIR, "test.jsonl")

    # Check data exists
    for f in [train_file, valid_file, test_file]:
        if not os.path.exists(f):
            print(f"Error: {f} not found. Run prepare_data.py first.")
            sys.exit(1)

    # Count training examples
    with open(train_file) as f:
        n_train = sum(1 for _ in f)
    print(f"Training examples: {n_train}")
    print(f"Model: {MODEL}")
    print(f"LoRA rank: {LORA_RANK}, layers: {LORA_LAYERS}")
    print(f"Epochs: {EPOCHS}, batch size: {BATCH_SIZE}")
    print(f"Output: {OUTPUT_DIR}")
    print()

    # Calculate total iterations
    iters_per_epoch = n_train // BATCH_SIZE
    total_iters = iters_per_epoch * EPOCHS
    print(f"Iterations per epoch: {iters_per_epoch}")
    print(f"Total iterations: {total_iters}")
    print()

    # Run mlx-lm fine-tuning
    cmd = [
        sys.executable, "-m", "mlx_lm.lora",
        "--model", MODEL,
        "--train",
        "--data", DATA_DIR,
        "--adapter-path", OUTPUT_DIR,
        "--lora-layers", str(LORA_LAYERS),
        "--lora-rank", str(LORA_RANK),
        "--batch-size", str(BATCH_SIZE),
        "--learning-rate", str(LEARNING_RATE),
        "--iters", str(total_iters),
        "--val-batches", "25",
        "--steps-per-eval", str(STEPS_PER_EVAL),
        "--save-every", str(SAVE_EVERY),
        "--max-seq-length", str(MAX_SEQ_LENGTH),
    ]

    print(f"Running: {' '.join(cmd)}")
    print("=" * 60)
    print()

    result = subprocess.run(cmd, cwd=os.path.dirname(__file__))

    if result.returncode != 0:
        print(f"\nTraining failed with exit code {result.returncode}")
        sys.exit(1)

    print()
    print("=" * 60)
    print(f"Training complete! Adapter saved to: {OUTPUT_DIR}")
    print()
    print("To test generation:")
    print(f"  python test_generate.py")
    print()
    print("To fuse the adapter into a full model:")
    print(f"  python -m mlx_lm.fuse --model {MODEL} --adapter-path {OUTPUT_DIR} --save-path adapters/pixl-fused")


if __name__ == "__main__":
    main()
