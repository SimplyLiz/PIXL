#!/usr/bin/env python3
"""
animate_water.py — Generate animated pixel art water with moonlight reflections.

Uses documented pixel art water animation techniques:
  1. Pixel circuit movement: water pixels shift 1px in a circuit (not whole rows)
  2. Highlight shimmer: bright reflection lines shift slowly left/right
  3. Waterfall flow: vertical bands darken/fragment as they descend
  4. Splash zone: synchronized foam particles, not horizontal shifts
  5. Moonlight reflection: subtle ripple offset with low amplitude

Reference: PIXL knowledge base — effects-fire-water-smoke.md,
           materials-surfaces-textures.md (section 5)
"""

import argparse
import math
from pathlib import Path

import numpy as np
from PIL import Image


def classify_pixels(arr: np.ndarray, water_start_y: int) -> dict:
    """Classify pixels into animation zones."""
    h, w = arr.shape[:2]

    # Find moon position (brightest pixel in top quarter)
    top = arr[:h // 4]
    bright = top[:, :, 0].astype(int) + top[:, :, 1].astype(int) + top[:, :, 2].astype(int)
    moon_y, moon_x = np.unravel_index(bright.argmax(), bright.shape)

    # Per-pixel brightness for classification
    pixel_bright = arr[:, :, 0].astype(int) + arr[:, :, 1].astype(int) + arr[:, :, 2].astype(int)

    # ── Classify water surface (below water_start_y) ──
    water = np.zeros((h, w), dtype=bool)
    for y in range(water_start_y, h):
        for x in range(w):
            r, g, b = int(arr[y, x, 0]), int(arr[y, x, 1]), int(arr[y, x, 2])
            water[y, x] = (b > r + 10) and (g > r) and (b > 40)

    # ── Classify waterfall (bright cyan/white vertical bands above water) ──
    waterfall = np.zeros((h, w), dtype=bool)
    for y in range(max(0, water_start_y - 80), water_start_y + 10):
        for x in range(w):
            r, g, b = int(arr[y, x, 0]), int(arr[y, x, 1]), int(arr[y, x, 2])
            total = r + g + b
            if total > 250 and g > r and b > r:
                waterfall[y, x] = True

    # ── Classify splash zone (water pixels near waterfall base) ──
    # Find the waterfall column range
    wf_cols = set()
    for y in range(max(0, water_start_y - 20), water_start_y + 5):
        for x in range(w):
            if waterfall[y, x]:
                wf_cols.add(x)

    wf_center = int(np.mean(list(wf_cols))) if wf_cols else w // 2
    wf_half_width = max(len(wf_cols) // 2, 10) if wf_cols else 20

    splash = np.zeros((h, w), dtype=bool)
    splash_depth = 25  # how many rows below water_start the splash extends
    for y in range(water_start_y, min(h, water_start_y + splash_depth)):
        spread = wf_half_width + (y - water_start_y) // 2  # widens slightly
        for x in range(max(0, wf_center - spread), min(w, wf_center + spread)):
            if water[y, x]:
                splash[y, x] = True

    # ── Classify highlight pixels on water (brighter than neighbors) ──
    highlight = np.zeros((h, w), dtype=bool)
    for y in range(water_start_y, h):
        for x in range(w):
            if not water[y, x] or splash[y, x]:
                continue
            b = pixel_bright[y, x]
            # A pixel is a "highlight" if it's notably brighter than the local area
            local_avg = 0
            count = 0
            for dy in range(-2, 3):
                for dx in range(-2, 3):
                    ny, nx = y + dy, x + dx
                    if 0 <= ny < h and 0 <= nx < w and water[ny, nx]:
                        local_avg += pixel_bright[ny, nx]
                        count += 1
            if count > 0:
                local_avg /= count
                if b > local_avg + 30:
                    highlight[y, x] = True

    # ── Moonlight reflection column ──
    reflection = np.zeros((h, w), dtype=bool)
    for y in range(water_start_y, h):
        if splash[y, moon_x if 0 <= moon_x < w else 0]:
            continue
        depth = y - water_start_y
        spread = 6 + depth // 12
        for x in range(max(0, moon_x - spread), min(w, moon_x + spread + 1)):
            if water[y, x] and not splash[y, x]:
                reflection[y, x] = True

    stats = {
        "water": water,
        "waterfall": waterfall,
        "splash": splash,
        "highlight": highlight,
        "reflection": reflection,
        "moon_x": int(moon_x),
        "moon_y": int(moon_y),
        "water_start_y": water_start_y,
        "wf_center": wf_center,
    }

    wc = int(water.sum())
    wfc = int(waterfall.sum())
    sc = int(splash.sum())
    hc = int(highlight.sum())
    rc = int(reflection.sum())
    print(f"  water={wc}  waterfall={wfc}  splash={sc}  highlight={hc}  reflection={rc}")
    print(f"  moon=({moon_x},{moon_y})  wf_center={wf_center}")

    return stats


def generate_frame(
    base: np.ndarray,
    masks: dict,
    frame_idx: int,
    total_frames: int,
) -> np.ndarray:
    """Generate one animation frame using proper pixel art water techniques."""
    frame = base.copy()
    h, w = frame.shape[:2]
    t = frame_idx / total_frames  # 0..1 normalized phase
    phase = t * 2 * math.pi

    water = masks["water"]
    waterfall = masks["waterfall"]
    splash = masks["splash"]
    highlight = masks["highlight"]
    reflection = masks["reflection"]
    water_start_y = masks["water_start_y"]
    moon_x = masks["moon_x"]

    # ── 1. Water highlight shimmer ──
    # Per the docs: "slowly shift highlight positions left/right, 4-8 frame cycle"
    # Only highlight pixels move, and only by 1px — everything else stays put
    for y in range(water_start_y, h):
        for x in range(w):
            if not highlight[y, x] or splash[y, x]:
                continue

            # Each highlight pixel shifts 1px left or right based on phase
            # Alternate direction per row for ripple pattern
            depth = y - water_start_y
            row_phase = phase + depth * 0.4
            shift = round(math.sin(row_phase))  # -1, 0, or 1

            nx = x + shift
            if 0 <= nx < w and water[y, nx]:
                # Move highlight: darken original, brighten target
                # Use the base (non-highlight) color for the vacated pixel
                # and the base + highlight delta for the new position
                for c in range(3):
                    # Restore this pixel toward its darker neighbor's value
                    neighbor_val = base[y, max(0, x - 1), c] if shift > 0 else base[y, min(w - 1, x + 1), c]
                    frame[y, x, c] = neighbor_val
                    # Brighten the target pixel
                    boost = int(base[y, x, c]) - int(neighbor_val)
                    val = int(base[y, nx, c]) + boost
                    frame[y, nx, c] = max(0, min(255, val))

    # ── 2. Moonlight reflection ripple ──
    # Per docs: "subtle ripple offset, low wave speed/width/height"
    # Brightness oscillation — NOT positional shifting
    for y in range(water_start_y, h):
        for x in range(w):
            if not reflection[y, x] or splash[y, x]:
                continue

            depth = y - water_start_y
            dist_from_center = abs(x - moon_x)

            # Gentle sine wave with downward phase propagation
            wave = math.sin(phase + depth * 0.25) * 0.12
            # Secondary harmonic for natural look
            wave += math.sin(phase * 1.7 + depth * 0.15 + dist_from_center * 0.08) * 0.06

            # Closer to center = stronger reflection
            center_falloff = max(0.3, 1.0 - dist_from_center / 15.0)
            # Deeper = weaker reflection
            depth_falloff = max(0.2, 1.0 - depth / 120.0)

            modulation = wave * center_falloff * depth_falloff

            for c in range(3):
                val = int(frame[y, x, c])
                val = int(val * (1.0 + modulation))
                frame[y, x, c] = max(0, min(255, val))

    # ── 3. Waterfall flow ──
    # Per docs: "vertical bands that sag and darken as they descend"
    # Subtle brightness flicker on waterfall pixels — NOT positional shifts
    rng = np.random.RandomState(frame_idx * 13 + 7)
    for y in range(h):
        for x in range(w):
            if not waterfall[y, x]:
                continue

            # ~25% of pixels flicker per frame for subtle spray
            if rng.random() > 0.25:
                continue

            # Vertical flow: pixels near top flicker brighter, near bottom dimmer
            noise = rng.randint(-12, 16)
            for c in range(3):
                val = int(frame[y, x, c]) + noise
                frame[y, x, c] = max(0, min(255, val))

    # ── 4. Splash zone ──
    # Per docs: "synchronized looping impact animation with foam particles"
    # Subtle brightness variation only — no horizontal movement
    for y in range(h):
        for x in range(w):
            if not splash[y, x]:
                continue

            depth = y - water_start_y
            dist_from_wf = abs(x - masks["wf_center"])

            # Foam particles: occasional bright flashes near the impact point
            foam_phase = phase + depth * 0.3 + dist_from_wf * 0.2
            foam = math.sin(foam_phase) * 0.08

            # Closer to impact = more active
            impact_strength = max(0, 1.0 - depth / 25.0) * max(0, 1.0 - dist_from_wf / 30.0)
            modulation = foam * impact_strength

            for c in range(3):
                val = int(frame[y, x, c])
                val = int(val * (1.0 + modulation))
                frame[y, x, c] = max(0, min(255, val))

    return frame


def auto_detect_water_y(arr: np.ndarray) -> int:
    """Find where the water surface begins."""
    h, w = arr.shape[:2]
    for y in range(h // 3, h):
        row = arr[y]
        blue_count = sum(
            1 for x in range(w)
            if int(row[x, 2]) > int(row[x, 0]) + 10
            and int(row[x, 1]) > int(row[x, 0])
            and int(row[x, 2]) > 40
        )
        if blue_count > w * 0.5:
            return y
    return int(h * 0.6)


def animate(
    input_path: str,
    output_path: str,
    num_frames: int = 8,
    frame_duration_ms: int = 120,
    water_start_y: int | None = None,
    scale: int = 1,
) -> str:
    """Generate animated water GIF from a pixelized image."""
    img = Image.open(input_path).convert("RGBA")
    arr = np.array(img)
    h, w = arr.shape[:2]

    if water_start_y is None:
        water_start_y = auto_detect_water_y(arr)

    print(f"Image: {w}x{h}, water starts at y={water_start_y}")

    masks = classify_pixels(arr, water_start_y)

    frames = []
    for i in range(num_frames):
        print(f"  frame {i + 1}/{num_frames}")
        frame_arr = generate_frame(arr, masks, i, num_frames)
        frame_img = Image.fromarray(frame_arr)

        if scale > 1:
            frame_img = frame_img.resize(
                (w * scale, h * scale), Image.Resampling.NEAREST
            )
        frames.append(frame_img)

    # Save GIF
    frames[0].save(
        output_path,
        save_all=True,
        append_images=frames[1:],
        duration=frame_duration_ms,
        loop=0,
        disposal=2,
    )
    print(f"Saved {num_frames}-frame GIF: {output_path}")

    # Save sprite sheet
    sheet_path = output_path.replace(".gif", "_sheet.png")
    fw = w * (scale if scale > 1 else 1)
    fh = h * (scale if scale > 1 else 1)
    sheet = Image.new("RGBA", (fw * num_frames, fh))
    for i, f in enumerate(frames):
        sheet.paste(f, (i * fw, 0))
    sheet.save(sheet_path)
    print(f"Saved sprite sheet: {sheet_path}")

    return output_path


def main():
    parser = argparse.ArgumentParser(
        description="Animate pixelized water scenes with moonlight reflections"
    )
    parser.add_argument("input", help="Input pixelized image (PNG)")
    parser.add_argument("-o", "--output", help="Output GIF path")
    parser.add_argument("-f", "--frames", type=int, default=8, help="Frame count (default: 8)")
    parser.add_argument("-d", "--duration", type=int, default=120, help="Frame duration ms (default: 120)")
    parser.add_argument("--water-y", type=int, help="Y where water starts (auto-detect if omitted)")
    parser.add_argument("-s", "--scale", type=int, default=1, help="Upscale factor")

    args = parser.parse_args()

    output = args.output
    if not output:
        p = Path(args.input)
        output = str(p.parent / f"{p.stem}_animated.gif")

    animate(
        args.input,
        output,
        num_frames=args.frames,
        frame_duration_ms=args.duration,
        water_start_y=args.water_y,
        scale=args.scale,
    )


if __name__ == "__main__":
    main()
