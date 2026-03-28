#!/bin/bash
set -e

cd "$(dirname "$0")"
source .venv/bin/activate

echo "=== PIXL LoRA Training v3 ==="
echo "Model: mlx-community/Qwen2.5-3B-Instruct-4bit"
echo ""

# Merge datasets: palette-matched + Cytopia
echo "Merging training data..."
mkdir -p data_v3

python -c "
import random
random.seed(42)

# Load both datasets
lines_matched = open('data_matched/train.jsonl').readlines() if __import__('os').path.exists('data_matched/train.jsonl') else []
lines_cytopia = open('data_cytopia/train.jsonl').readlines() if __import__('os').path.exists('data_cytopia/train.jsonl') else []
print(f'  Palette-matched: {len(lines_matched)} train pairs')
print(f'  Cytopia:         {len(lines_cytopia)} train pairs')

# Combine and shuffle
all_train = lines_matched + lines_cytopia
random.shuffle(all_train)
with open('data_v3/train.jsonl', 'w') as f:
    f.writelines(all_train)

# Same for valid/test
for split in ['valid', 'test']:
    lines = []
    for src in ['data_matched', 'data_cytopia']:
        path = f'{src}/{split}.jsonl'
        if __import__('os').path.exists(path):
            lines.extend(open(path).readlines())
    random.shuffle(lines)
    with open(f'data_v3/{split}.jsonl', 'w') as f:
        f.writelines(lines)
    print(f'  {split}: {len(lines)} pairs')

print(f'  Total train: {len(all_train)} pairs')
"

echo ""
echo "Starting LoRA fine-tuning..."

mkdir -p adapters/pixl-lora-v3

python -m mlx_lm lora \
  --model mlx-community/Qwen2.5-3B-Instruct-4bit \
  --train \
  --data data_v3 \
  --adapter-path adapters/pixl-lora-v3 \
  --fine-tune-type lora \
  --num-layers 16 \
  --batch-size 1 \
  --learning-rate 2e-5 \
  --iters 30000 \
  --val-batches 25 \
  --steps-per-eval 500 \
  --save-every 2000 \
  --max-seq-length 512 \
  --seed 42

echo ""
echo "=== Training complete ==="
echo "Adapter: adapters/pixl-lora-v3/"
echo "Update Studio Settings > PIXL LoRA > Adapter Path to: training/adapters/pixl-lora-v3"
