#!/usr/bin/env python3
"""PIXL Map Generator training script.

Replaces train_me.sh with Python, adding throttle support so your Mac
stays responsive during training.

Usage:
    python train_me.py                  # throttled (default), resume auto-detected
    python train_me.py --unthrottle     # full speed, all resources
    python train_me.py --epoch 2        # label this as epoch 2 of 5
    python train_me.py --delay 0.2      # custom throttle delay (seconds per step)
"""

import argparse
import os
import signal
import sys
import time
import types
from pathlib import Path


TOTAL_EPOCHS = 5
CONFIG_FILE = "lora_config_mapgen.yaml"
ADAPTER_DIR = "adapters/pixl-mapgen"
ADAPTER_FILE = f"{ADAPTER_DIR}/adapters.safetensors"
DEFAULT_THROTTLE_DELAY = 0.1  # seconds to sleep per training step


def parse_args():
    p = argparse.ArgumentParser(description="PIXL Map Generator trainer")
    p.add_argument("--epoch", type=int, default=1, help="Current epoch (1-based)")
    p.add_argument(
        "--unthrottle",
        action="store_true",
        help="Run at full speed (default is throttled)",
    )
    p.add_argument(
        "--delay",
        type=float,
        default=DEFAULT_THROTTLE_DELAY,
        help=f"Throttle delay in seconds per step (default: {DEFAULT_THROTTLE_DELAY})",
    )
    return p.parse_args()


def count_lines(path: str) -> int:
    with open(path) as f:
        return sum(1 for _ in f)


def ensure_venv():
    """Re-exec under .venv/bin/python3 if it exists and we're not already in it."""
    venv_python = Path(__file__).parent / ".venv" / "bin" / "python3"
    if venv_python.is_file() and sys.prefix == sys.base_prefix:
        # sys.prefix != sys.base_prefix when already inside a venv
        print(f"[venv] Re-launching under {venv_python}")
        os.execv(str(venv_python), [str(venv_python)] + sys.argv)


def main():
    ensure_venv()
    args = parse_args()
    throttled = not args.unthrottle

    os.chdir(Path(__file__).parent)

    # Lower process priority when throttled
    if throttled:
        os.nice(10)
        print(f"[throttle] Process niced +10, {args.delay:.0f}ms delay/step")
        print(f"[throttle] Run with --unthrottle for full speed")

    import yaml
    import numpy as np
    import mlx.core as mx
    from mlx.utils import tree_flatten
    from mlx_lm.tuner.callbacks import TrainingCallback
    from mlx_lm.lora import (
        build_parser,
        load,
        load_dataset,
        train_model,
        get_reporting_callbacks,
        CONFIG_DEFAULTS,
    )

    # Load config
    with open(CONFIG_FILE) as f:
        yaml_config = yaml.safe_load(f)

    # Build args namespace like mlx_lm expects
    parser = build_parser()
    lm_args = parser.parse_args(["-c", CONFIG_FILE])
    cfg = vars(lm_args)
    for k, v in yaml_config.items():
        if cfg.get(k) is None:
            cfg[k] = v
    for k, v in CONFIG_DEFAULTS.items():
        if cfg.get(k) is None:
            cfg[k] = v

    # Resume from checkpoint if exists
    completed_iters = 0
    if Path(ADAPTER_FILE).exists():
        cfg["resume_adapter_file"] = ADAPTER_FILE
        # Detect last checkpoint iteration from numbered files like 0004000_adapters.safetensors
        checkpoint_iters = sorted(
            int(p.name.split("_")[0])
            for p in Path(ADAPTER_DIR).glob("[0-9]*_adapters.safetensors")
        )
        if checkpoint_iters:
            completed_iters = checkpoint_iters[-1]
        print(f"Resuming from {ADAPTER_FILE} (iter {completed_iters}/{cfg.get('iters', '?')})")
        if completed_iters > 0:
            remaining = cfg["iters"] - completed_iters
            if remaining <= 0:
                print(f"Already completed {completed_iters} iters — nothing to do.")
                sys.exit(0)
            cfg["iters"] = remaining

    # Throttle: report every step so callback fires each iteration
    if throttled:
        cfg["steps_per_report"] = 1

    cfg = types.SimpleNamespace(**cfg)

    # Count data
    train_count = count_lines("data_me/train.jsonl")
    valid_count = count_lines("data_me/valid.jsonl")

    print(f"=== PIXL Map Generator — Epoch {args.epoch}/{TOTAL_EPOCHS} ===")
    print(f"Model: {cfg.model}")
    print(f"Data: data_me/ ({train_count} train, {valid_count} valid)")
    print(f"LoRA: rank={cfg.lora_rank}, layers={cfg.lora_layers}, lr={cfg.learning_rate}")
    print(f"Checkpoints every {cfg.save_every} iters — Ctrl+C safe after first checkpoint")
    print(f"Mode: {'THROTTLED' if throttled else 'UNTHROTTLED'}")
    print()

    Path(ADAPTER_DIR).mkdir(parents=True, exist_ok=True)

    np.random.seed(cfg.seed)

    # Build callback chain: reporting + optional throttle
    reporting_cb = get_reporting_callbacks(
        cfg.report_to,
        project_name=cfg.project_name,
        log_dir=cfg.adapter_path,
        config=vars(cfg),
    )

    class ThrottledCallback(TrainingCallback):
        """Wraps the reporting callback and adds a sleep for throttling."""

        def __init__(self, inner, delay: float):
            self.inner = inner
            self.delay = delay
            self._last_iter = 0

        def on_train_loss_report(self, train_info: dict):
            if self.inner is not None:
                self.inner.on_train_loss_report(train_info)
            if self.delay > 0:
                time.sleep(self.delay)
            self._last_iter = train_info.get("iteration", 0)

        def on_val_loss_report(self, val_info: dict):
            if self.inner is not None:
                self.inner.on_val_loss_report(val_info)

    callback = ThrottledCallback(reporting_cb, args.delay) if throttled else reporting_cb

    print("Loading pretrained model")
    model, tokenizer = load(cfg.model, tokenizer_config={"trust_remote_code": True})

    print("Loading datasets")
    train_set, valid_set, test_set = load_dataset(cfg, tokenizer)

    print("Training")
    try:
        train_model(cfg, model, train_set, valid_set, callback)
        print(f"Training complete. Adapter: {cfg.adapter_path}")
    except KeyboardInterrupt:
        print("\n")
        print("Interrupted — saving checkpoint...")
        weights = dict(tree_flatten(model.trainable_parameters()))
        adapter_file = Path(cfg.adapter_path) / "adapters.safetensors"
        mx.save_safetensors(str(adapter_file), weights)
        print(f"Checkpoint saved to {adapter_file}")
        print("Resume with: python train_me.py")
        sys.exit(0)

    print()
    print(f"=== Epoch {args.epoch} complete ===")
    print(f"Adapter saved: {ADAPTER_DIR}/")

    if args.epoch < TOTAL_EPOCHS:
        nxt = args.epoch + 1
        print()
        print(f"To continue: python train_me.py --epoch {nxt}")
        print(f"To test now: .venv/bin/python generate_map.py --prompt 'open dungeon' --theme dark_fantasy")
    else:
        print(f"All {TOTAL_EPOCHS} epochs done!")


if __name__ == "__main__":
    main()
