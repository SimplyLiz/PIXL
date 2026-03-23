#!/bin/bash
set -e

cd "$(dirname "$0")"
source .venv/bin/activate

echo "=== PIXL LoRA Training (Palette-Matched) ==="
echo "Model: mlx-community/Qwen2.5-3B-Instruct-4bit"
echo "Data: data_matched/ (8877 train, 493 valid, 494 test)"
echo "Augmentation: 4x (rotations)"
echo ""

mkdir -p adapters/pixl-lora-v2

# 5 epochs on 8877 samples = 44385 iters (too many for M4 Pro)
# Use 3 epochs = 26631 iters, save every 1000
# At ~2 it/sec = ~3.5 hours

python -m mlx_lm lora \
  --model mlx-community/Qwen2.5-3B-Instruct-4bit \
  --train \
  --data data_matched \
  --adapter-path adapters/pixl-lora-v2 \
  --fine-tune-type lora \
  --num-layers 16 \
  --batch-size 1 \
  --learning-rate 2e-5 \
  --iters 26631 \
  --val-batches 25 \
  --steps-per-eval 500 \
  --save-every 2000 \
  --max-seq-length 512 \
  --seed 42

echo ""
echo "=== Training complete ==="
echo "Adapter: adapters/pixl-lora-v2/"
