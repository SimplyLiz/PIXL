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
    "a 16x16 pixel art tile (obstacle type)",
    "a 16x16 pixel art tile (walkable type)",
    "a 16x16 pixel art tile (hazard type)",
    "a 16x16 pixel art tile tagged: stone, wall",
    "a 16x16 pixel art tile tagged: grass, floor",
]


def main():
    from mlx_lm import load, generate

    print(f"Loading model: {MODEL}")
    print(f"Adapter: {ADAPTER_PATH}")

    model, tokenizer = load(
        MODEL,
        adapter_path=ADAPTER_PATH if os.path.exists(ADAPTER_PATH) else None,
    )

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
            temp=0.3,
        )

        print(response)
        print()


if __name__ == "__main__":
    main()
