#!/bin/bash
set -e

cd "$(dirname "$0")"
source .venv/bin/activate

TRAIN_COUNT=$(wc -l < data_me/train.jsonl | tr -d ' ')
VALID_COUNT=$(wc -l < data_me/valid.jsonl | tr -d ' ')
EPOCH=${1:-1}
TOTAL_EPOCHS=5

SAVE_EVERY=$(grep 'save_every' lora_config_mapgen.yaml | awk '{print $2}')
echo "=== PIXL Map Generator — Epoch $EPOCH/$TOTAL_EPOCHS ==="
echo "Model: mlx-community/Qwen2.5-3B-Instruct-4bit"
echo "Data: data_me/ ($TRAIN_COUNT train, $VALID_COUNT valid)"
echo "LoRA: rank=16, layers=24, lr=2e-5"
echo "Checkpoints every $SAVE_EVERY iters — Ctrl+C safe after first checkpoint"
echo ""

mkdir -p adapters/pixl-mapgen

# Check for existing adapter to resume from
RESUME_FLAG=""
if [ -f "adapters/pixl-mapgen/adapters.safetensors" ]; then
    echo "Resuming from existing adapter checkpoint"
    RESUME_FLAG="--resume-adapter-file adapters/pixl-mapgen/adapters.safetensors"
fi

python train_runner.py \
  -c lora_config_mapgen.yaml \
  --iters "$TRAIN_COUNT" \
  $RESUME_FLAG

echo ""
echo "=== Epoch $EPOCH complete ==="
echo "Adapter saved: adapters/pixl-mapgen/"

if [ "$EPOCH" -lt "$TOTAL_EPOCHS" ]; then
    NEXT=$((EPOCH + 1))
    echo ""
    echo "To continue: bash train_me.sh $NEXT"
    echo "To test now: .venv/bin/python generate_map.py --prompt 'open dungeon' --theme dark_fantasy"
else
    echo "All $TOTAL_EPOCHS epochs done!"
fi
