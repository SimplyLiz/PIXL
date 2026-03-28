#!/usr/bin/env python3
"""
pixelize.py — Convert fake pixel art (AI-generated) to true 1:1 pixel art.

Takes a high-res image that mimics pixel art style and produces a clean,
grid-aligned, palette-quantized PNG at the target resolution.
"""

import argparse
import sys
from pathlib import Path

import numpy as np
from PIL import Image


def quantize_palette(img: Image.Image, num_colors: int) -> Image.Image:
    """Reduce image to N colors using median-cut quantization, return as RGB."""
    # Convert to P mode (palette) with median cut
    quantized = img.quantize(colors=num_colors, method=Image.Quantize.MEDIANCUT)
    return quantized.convert("RGB")


def downsample(img: Image.Image, target_w: int, target_h: int) -> Image.Image:
    """Downsample using LANCZOS (best for downscaling), then snap via nearest."""
    # First pass: high-quality downsample to get good color averaging
    small = img.resize((target_w, target_h), Image.Resampling.LANCZOS)
    return small


def pixelize(
    input_path: str,
    output_path: str | None = None,
    target_width: int | None = None,
    target_height: int | None = None,
    scale_factor: int | None = None,
    num_colors: int = 32,
    upscale: int = 1,
) -> str:
    """
    Convert a fake pixel art image to true 1:1 pixel art.

    Args:
        input_path: Path to input image
        output_path: Path for output (auto-generated if None)
        target_width: Target pixel art width (mutually exclusive with scale_factor)
        target_height: Target pixel art height (auto-calculated from width if None)
        scale_factor: Divide source dimensions by this (mutually exclusive with target_width)
        num_colors: Max palette colors (0 = skip quantization)
        upscale: Integer upscale factor for the output (for preview, e.g., 4x)

    Returns:
        Path to the output file
    """
    img = Image.open(input_path).convert("RGB")
    src_w, src_h = img.size
    aspect = src_h / src_w

    # Determine target dimensions
    if scale_factor:
        tw = src_w // scale_factor
        th = src_h // scale_factor
    elif target_width:
        tw = target_width
        th = target_height or round(target_width * aspect)
    else:
        # Default: try to get to ~128-256px wide
        tw = 160
        th = round(160 * aspect)

    print(f"Source: {src_w}x{src_h} → Target: {tw}x{th} ({num_colors} colors)")

    # Step 1: Downsample
    result = downsample(img, tw, th)

    # Step 2: Palette quantization
    if num_colors > 0:
        result = quantize_palette(result, num_colors)

    # Step 3: Optional upscale for preview (nearest-neighbor to keep crisp)
    if upscale > 1:
        result = result.resize(
            (tw * upscale, th * upscale), Image.Resampling.NEAREST
        )
        suffix = f"_pixel_{tw}x{th}_{num_colors}c_preview{upscale}x"
    else:
        suffix = f"_pixel_{tw}x{th}_{num_colors}c"

    # Output path
    if not output_path:
        p = Path(input_path)
        output_path = str(p.parent / f"{p.stem}{suffix}{p.suffix}")

    result.save(output_path)
    print(f"Saved: {output_path}")
    return output_path


def main():
    parser = argparse.ArgumentParser(description="Convert fake pixel art to true 1:1 pixel art")
    parser.add_argument("input", help="Input image path")
    parser.add_argument("-o", "--output", help="Output path (auto-generated if omitted)")
    parser.add_argument("-w", "--width", type=int, help="Target pixel art width")
    parser.add_argument("--height", type=int, help="Target pixel art height")
    parser.add_argument("-s", "--scale", type=int, help="Downscale factor (e.g., 8 = divide by 8)")
    parser.add_argument("-c", "--colors", type=int, default=32, help="Palette color count (0=skip, default=32)")
    parser.add_argument("-u", "--upscale", type=int, default=1, help="Preview upscale factor (e.g., 4)")

    args = parser.parse_args()

    if args.width and args.scale:
        print("Error: specify --width or --scale, not both", file=sys.stderr)
        sys.exit(1)

    pixelize(
        args.input,
        output_path=args.output,
        target_width=args.width,
        target_height=args.height,
        scale_factor=args.scale,
        num_colors=args.colors,
        upscale=args.upscale,
    )


if __name__ == "__main__":
    main()
