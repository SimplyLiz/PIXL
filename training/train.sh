#!/bin/bash
set -e

cd "$(dirname "$0")"
source .venv/bin/activate

echo "=== PIXL LoRA Training ==="
echo "Model: mlx-community/Qwen2.5-3B-Instruct-4bit"
echo "Data: data/ (2386 train, 133 valid, 133 test)"
echo "Output: adapters/pixl-lora/"
echo ""

mkdir -p adapters/pixl-lora

python -m mlx_lm lora \
  --model mlx-community/Qwen2.5-3B-Instruct-4bit \
  --train \
  --data data \
  --adapter-path adapters/pixl-lora \
  --fine-tune-type lora \
  --num-layers 16 \
  --batch-size 1 \
  --learning-rate 1e-5 \
  --iters 7158 \
  --val-batches 25 \
  --steps-per-eval 100 \
  --save-every 500 \
  --max-seq-length 512 \
  --seed 42

echo ""
echo "=== Training complete ==="
echo "Adapter: adapters/pixl-lora/"
echo "Run: python test_generate.py"
