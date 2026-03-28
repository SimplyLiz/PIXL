"""Test the fine-tuned LoRA model by generating PAX grids."""

import os

MODEL = "mlx-community/Qwen2.5-3B-Instruct-4bit"
ADAPTER_PATH = os.path.join(os.path.dirname(__file__), "adapters", "pixl-lora")

SYSTEM_PROMPT = """You are a pixel art tile generator. Given a description, output a PAX-format character grid.
Rules:
- Use only the symbols from the palette provided
- Each row must be exactly the specified width
- Total rows must equal the specified height
- '.' means transparent/void
- Output ONLY the grid, no explanation"""

TEST_PROMPTS = [
    # Legacy prompts (v1 format)
    "a 16x16 pixel art tile (obstacle type)",
    "a 16x16 pixel art tile (walkable type)",
    "a 16x16 pixel art tile tagged: stone, wall",
    "a 16x16 pixel art tile tagged: grass, floor",
    # Rich feature-based prompts (v2 format — matches prepare_matched.py labels)
    "Palette: '#'=(32,32,48) '+'=(64,64,80) '='=(96,96,112) '~'=(128,128,144)\ntileset:Terrain, density:dense, symmetry:low, detail:moderate, colors:few",
    "Palette: '#'=(16,48,16) '+'=(32,96,32) '='=(64,144,64) '~'=(96,192,96)\ntileset:Nature, density:solid, symmetry:medium, detail:simple, colors:few",
    "Palette: '#'=(48,32,16) '+'=(96,64,32) '='=(144,112,80) '~'=(192,176,144)\ntileset:Items, density:sparse, symmetry:high, detail:complex, colors:few",
]


def main():
    from mlx_lm import load, generate

    print(f"Loading model: {MODEL}")
    adapter = ADAPTER_PATH if os.path.exists(ADAPTER_PATH) else None
    print(f"Adapter: {adapter}")

    model, tokenizer = load(MODEL, adapter_path=adapter)

    for prompt in TEST_PROMPTS:
        print(f"\n{'='*60}")
        print(f"Prompt: {prompt}")
        print(f"{'='*60}")

        messages = [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": prompt},
        ]

        formatted = tokenizer.apply_chat_template(
            messages, tokenize=False, add_generation_prompt=True
        )

        response = generate(
            model,
            tokenizer,
            prompt=formatted,
            max_tokens=512,
            verbose=False,
        )

        print(response)
        print()


if __name__ == "__main__":
    main()
