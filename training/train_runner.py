"""Training runner with Ctrl+C checkpoint support.

Wraps mlx_lm's LoRA training so that SIGINT (Ctrl+C) saves a
checkpoint before exiting, instead of losing all progress.

Usage:
    python train_runner.py -c lora_config_mapgen.yaml [--iters N] [--resume-adapter-file PATH]
"""

import sys
import types
from pathlib import Path


def main():
    import yaml
    import numpy as np
    import mlx.core as mx
    from mlx.utils import tree_flatten
    from mlx_lm.lora import (
        build_parser,
        run,
        load,
        load_dataset,
        train_model,
        get_reporting_callbacks,
        CONFIG_DEFAULTS,
    )

    # Parse args + load config (same logic as mlx_lm.lora.main)
    parser = build_parser()
    args = parser.parse_args()
    config = args.config
    args = vars(args)
    if config:
        print("Loading configuration file", config)
        with open(config, "r") as f:
            yaml_config = yaml.safe_load(f)
        for k, v in yaml_config.items():
            if args.get(k, None) is None:
                args[k] = v
    for k, v in CONFIG_DEFAULTS.items():
        if args.get(k, None) is None:
            args[k] = v
    args = types.SimpleNamespace(**args)

    # Run training with KeyboardInterrupt handler
    np.random.seed(args.seed)

    training_callback = get_reporting_callbacks(
        args.report_to,
        project_name=args.project_name,
        log_dir=args.adapter_path,
        config=vars(args),
    )

    print("Loading pretrained model")
    model, tokenizer = load(args.model, tokenizer_config={"trust_remote_code": True})

    print("Loading datasets")
    train_set, valid_set, test_set = load_dataset(args, tokenizer)

    print("Training")
    try:
        train_model(args, model, train_set, valid_set, training_callback)
        print(f"Training complete. Adapter: {args.adapter_path}")
    except KeyboardInterrupt:
        print("\n")
        print("Interrupted — saving checkpoint...")
        weights = dict(tree_flatten(model.trainable_parameters()))
        adapter_file = Path(args.adapter_path) / "adapters.safetensors"
        mx.save_safetensors(str(adapter_file), weights)
        print(f"Checkpoint saved to {adapter_file}")
        print("Resume with: bash train_me.sh")
        sys.exit(0)


if __name__ == "__main__":
    main()
