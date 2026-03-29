#!/usr/bin/env python3
"""
Train a convolutional autoencoder on Eye of the Beholder wall/ceiling tiles.

Workflow:
  1. Slice sprite sheets into individual tiles (auto-detect via cyan key)
  2. Normalize to uniform size, generate metadata JSON
  3. Augment (horizontal flip, palette jitter)
  4. Train a small conv autoencoder to learn the latent style space
  5. Save model + encoder for downstream use (interpolation, generation)

Usage:
  python tool/scripts/train_tiles.py                    # full pipeline
  python tool/scripts/train_tiles.py --slice-only        # just extract tiles
  python tool/scripts/train_tiles.py --train-only        # skip slicing, train
  python tool/scripts/train_tiles.py --epochs 200        # custom epoch count
"""

import argparse
import json
import os
import sys
from datetime import datetime
from pathlib import Path

import numpy as np
from PIL import Image

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

PROJECT_ROOT = Path(__file__).resolve().parents[2]
REFERENCE_DIR = PROJECT_ROOT / "reference" / "eotb-sprites"
EOTB1_WALLS = REFERENCE_DIR / "eotb1" / "walls"
EOTB1_FLOORS = REFERENCE_DIR / "eotb1" / "floors_ceilings"
OUTPUT_DIR = REFERENCE_DIR / "dataset"
SLICED_DIR = OUTPUT_DIR / "sliced"
AUGMENTED_DIR = OUTPUT_DIR / "augmented"
MODEL_DIR = OUTPUT_DIR / "model"
METADATA_PATH = OUTPUT_DIR / "metadata.json"

# Background/transparency key colors (detected from actual sprite sheets)
BG_COLORS = [
    (87, 255, 255),   # light cyan used in Spriters Resource sheets
    (0, 255, 255),    # standard cyan key
    (255, 0, 255),    # magenta key (secondary transparency)
    (0, 128, 128),    # dark teal separator in floors/ceilings sheet
]
MIN_TILE_SIZE = 8         # ignore regions smaller than 8x8
TARGET_SIZE = 64           # normalize all tiles to 64x64 for training

# ---------------------------------------------------------------------------
# Step 1: Slice sprite sheets into individual tiles
# ---------------------------------------------------------------------------

def is_bg_pixel(img_array: np.ndarray) -> np.ndarray:
    """Return boolean mask of pixels matching any background/transparency color."""
    mask = np.zeros(img_array.shape[:2], dtype=bool)
    for color in BG_COLORS:
        mask |= (
            (img_array[:, :, 0] == color[0]) &
            (img_array[:, :, 1] == color[1]) &
            (img_array[:, :, 2] == color[2])
        )
    return mask


def find_tile_bboxes(img_array: np.ndarray) -> list[tuple[int, int, int, int]]:
    """Find bounding boxes by detecting background gutter rows/columns that separate tiles."""
    h, w = img_array.shape[:2]

    is_cyan = is_bg_pixel(img_array)

    # Find rows/cols that are mostly cyan (>95%) — these are gutters
    cyan_threshold = 0.95
    row_cyan_frac = is_cyan.mean(axis=1)  # fraction of cyan per row
    col_cyan_frac = is_cyan.mean(axis=0)  # fraction of cyan per col

    # Find contiguous non-gutter bands
    def find_bands(frac: np.ndarray, min_size: int) -> list[tuple[int, int]]:
        """Return (start, end) ranges of contiguous non-gutter regions."""
        in_band = frac < cyan_threshold
        bands = []
        start = None
        for i in range(len(in_band)):
            if in_band[i] and start is None:
                start = i
            elif not in_band[i] and start is not None:
                if i - start >= min_size:
                    bands.append((start, i))
                start = None
        if start is not None and len(in_band) - start >= min_size:
            bands.append((start, len(in_band)))
        return bands

    row_bands = find_bands(row_cyan_frac, MIN_TILE_SIZE)
    col_bands = find_bands(col_cyan_frac, MIN_TILE_SIZE)

    # Each intersection of a row band and col band is a potential tile
    bboxes = []
    for y_start, y_end in row_bands:
        for x_start, x_end in col_bands:
            region = img_array[y_start:y_end, x_start:x_end]
            region_cyan = is_cyan[y_start:y_end, x_start:x_end]

            # Trim cyan borders within the cell to get tight bbox
            non_cyan_rows = np.where(~region_cyan.all(axis=1))[0]
            non_cyan_cols = np.where(~region_cyan.all(axis=0))[0]

            if len(non_cyan_rows) == 0 or len(non_cyan_cols) == 0:
                continue

            ty = y_start + non_cyan_rows[0]
            by = y_start + non_cyan_rows[-1] + 1
            tx = x_start + non_cyan_cols[0]
            bx = x_start + non_cyan_cols[-1] + 1

            tw = bx - tx
            th = by - ty

            if tw >= MIN_TILE_SIZE and th >= MIN_TILE_SIZE:
                bboxes.append((tx, ty, tw, th))

    return bboxes


def slice_sheet(sheet_path: Path, output_dir: Path, source_tag: str) -> list[dict]:
    """Slice a sprite sheet into individual tiles. Returns metadata entries."""
    img = Image.open(sheet_path).convert("RGB")
    arr = np.array(img)

    print(f"  Scanning {sheet_path.name} ({arr.shape[1]}x{arr.shape[0]})...")
    bboxes = find_tile_bboxes(arr)

    # Sort top-to-bottom, left-to-right
    bboxes.sort(key=lambda b: (b[1] // 40, b[0]))

    print(f"  Found {len(bboxes)} tiles")

    entries = []
    for i, (x, y, w, h) in enumerate(bboxes):
        tile = img.crop((x, y, x + w, y + h))

        # Replace any remaining background key colors with black
        tile_arr = np.array(tile)
        cyan_mask = is_bg_pixel(tile_arr)
        tile_arr[cyan_mask] = [0, 0, 0]
        tile = Image.fromarray(tile_arr)

        stem = sheet_path.stem.replace(" ", "_").lower()
        filename = f"{stem}_tile_{i:03d}.png"
        tile.save(output_dir / filename)

        entries.append({
            "filename": filename,
            "source_sheet": sheet_path.name,
            "source_tag": source_tag,
            "category": "wall" if "wall" in source_tag.lower() else "floor_ceiling",
            "original_size": [int(w), int(h)],
            "bbox": [int(x), int(y), int(w), int(h)],
            "tile_index": i,
            "color_count": len(set(tuple(p) for p in np.array(tile).reshape(-1, 3).tolist())),
            "has_transparency": bool(np.any(cyan_mask)),
            "mean_luminance": float(np.mean(
                0.299 * tile_arr[:, :, 0] +
                0.587 * tile_arr[:, :, 1] +
                0.114 * tile_arr[:, :, 2]
            )),
        })

    return entries


def run_slicing():
    """Slice all sprite sheets and write metadata."""
    SLICED_DIR.mkdir(parents=True, exist_ok=True)

    all_entries = []

    # Walls
    if EOTB1_WALLS.exists():
        for sheet in sorted(EOTB1_WALLS.glob("*.png")):
            tag = sheet.stem.replace("level_", "").replace("_walls", "")
            entries = slice_sheet(sheet, SLICED_DIR, source_tag=f"walls_{tag}")
            all_entries.extend(entries)

    # Floors & ceilings
    if EOTB1_FLOORS.exists():
        for sheet in sorted(EOTB1_FLOORS.glob("*.png")):
            entries = slice_sheet(sheet, SLICED_DIR, source_tag="floors_ceilings")
            all_entries.extend(entries)

    # Write metadata
    metadata = {
        "created": datetime.now().isoformat(),
        "total_tiles": len(all_entries),
        "categories": {},
        "tiles": all_entries,
    }
    for e in all_entries:
        cat = e["category"]
        metadata["categories"][cat] = metadata["categories"].get(cat, 0) + 1

    METADATA_PATH.parent.mkdir(parents=True, exist_ok=True)
    with open(METADATA_PATH, "w") as f:
        json.dump(metadata, f, indent=2)

    print(f"\nSliced {len(all_entries)} tiles total:")
    for cat, count in metadata["categories"].items():
        print(f"  {cat}: {count}")
    print(f"Metadata: {METADATA_PATH}")

    return all_entries


# ---------------------------------------------------------------------------
# Step 2: Augment dataset
# ---------------------------------------------------------------------------

def augment_tiles(entries: list[dict] | None = None):
    """Create augmented versions: horizontal flip + small palette shifts."""
    AUGMENTED_DIR.mkdir(parents=True, exist_ok=True)

    if entries is None:
        with open(METADATA_PATH) as f:
            entries = json.load(f)["tiles"]

    augmented = []
    for e in entries:
        src = SLICED_DIR / e["filename"]
        if not src.exists():
            continue

        img = Image.open(src).convert("RGB")

        # Resize to target with nearest neighbor (preserve pixel art)
        img_resized = img.resize((TARGET_SIZE, TARGET_SIZE), Image.NEAREST)

        stem = Path(e["filename"]).stem

        # Original (resized)
        out_name = f"{stem}.png"
        img_resized.save(AUGMENTED_DIR / out_name)
        augmented.append(out_name)

        # Horizontal flip
        flipped = img_resized.transpose(Image.FLIP_LEFT_RIGHT)
        out_name = f"{stem}_flip.png"
        flipped.save(AUGMENTED_DIR / out_name)
        augmented.append(out_name)

        # Warm palette shift (+10 red, -5 blue)
        arr = np.array(img_resized, dtype=np.int16)
        arr[:, :, 0] = np.clip(arr[:, :, 0] + 10, 0, 255)
        arr[:, :, 2] = np.clip(arr[:, :, 2] - 5, 0, 255)
        warm = Image.fromarray(arr.astype(np.uint8))
        out_name = f"{stem}_warm.png"
        warm.save(AUGMENTED_DIR / out_name)
        augmented.append(out_name)

        # Cool palette shift (-5 red, +10 blue)
        arr = np.array(img_resized, dtype=np.int16)
        arr[:, :, 0] = np.clip(arr[:, :, 0] - 5, 0, 255)
        arr[:, :, 2] = np.clip(arr[:, :, 2] + 10, 0, 255)
        cool = Image.fromarray(arr.astype(np.uint8))
        out_name = f"{stem}_cool.png"
        cool.save(AUGMENTED_DIR / out_name)
        augmented.append(out_name)

    print(f"\nAugmented: {len(entries)} originals -> {len(augmented)} samples")
    return augmented


# ---------------------------------------------------------------------------
# Step 3: Train convolutional autoencoder
# ---------------------------------------------------------------------------

def load_dataset() -> np.ndarray:
    """Load all augmented tiles as a numpy array normalized to [0, 1]."""
    images = []
    for p in sorted(AUGMENTED_DIR.glob("*.png")):
        img = Image.open(p).convert("RGB")
        arr = np.array(img, dtype=np.float32) / 255.0
        images.append(arr)

    data = np.array(images)
    # channels-last: (N, H, W, 3)
    print(f"Dataset shape: {data.shape}")
    return data


def build_autoencoder(latent_dim: int = 32):
    """Build a small convolutional autoencoder. Returns (autoencoder, encoder)."""
    try:
        import torch
        import torch.nn as nn
    except ImportError:
        print("ERROR: PyTorch is required. Install with: pip install torch torchvision")
        sys.exit(1)

    class Encoder(nn.Module):
        def __init__(self):
            super().__init__()
            self.conv = nn.Sequential(
                nn.Conv2d(3, 32, 3, stride=2, padding=1),   # 64 -> 32
                nn.ReLU(),
                nn.Conv2d(32, 64, 3, stride=2, padding=1),  # 32 -> 16
                nn.ReLU(),
                nn.Conv2d(64, 128, 3, stride=2, padding=1), # 16 -> 8
                nn.ReLU(),
                nn.Conv2d(128, 256, 3, stride=2, padding=1), # 8 -> 4
                nn.ReLU(),
            )
            self.fc = nn.Linear(256 * 4 * 4, latent_dim)

        def forward(self, x):
            x = self.conv(x)
            x = x.reshape(x.size(0), -1)
            return self.fc(x)

    class Decoder(nn.Module):
        def __init__(self):
            super().__init__()
            self.fc = nn.Linear(latent_dim, 256 * 4 * 4)
            self.deconv = nn.Sequential(
                nn.ConvTranspose2d(256, 128, 4, stride=2, padding=1), # 4 -> 8
                nn.ReLU(),
                nn.ConvTranspose2d(128, 64, 4, stride=2, padding=1),  # 8 -> 16
                nn.ReLU(),
                nn.ConvTranspose2d(64, 32, 4, stride=2, padding=1),   # 16 -> 32
                nn.ReLU(),
                nn.ConvTranspose2d(32, 3, 4, stride=2, padding=1),    # 32 -> 64
                nn.Sigmoid(),
            )

        def forward(self, x):
            x = self.fc(x)
            x = x.reshape(x.size(0), 256, 4, 4)
            return self.deconv(x)

    class Autoencoder(nn.Module):
        def __init__(self):
            super().__init__()
            self.encoder = Encoder()
            self.decoder = Decoder()

        def forward(self, x):
            z = self.encoder(x)
            return self.decoder(z), z

    model = Autoencoder()
    return model


def train(epochs: int = 100, lr: float = 1e-3, batch_size: int = 16, resume: bool = False):
    """Train the autoencoder on the augmented dataset."""
    import torch
    import torch.nn as nn
    from torch.utils.data import DataLoader, TensorDataset

    device = torch.device("mps" if torch.backends.mps.is_available() else "cpu")
    print(f"Device: {device}")

    # Load data: (N, H, W, C) -> (N, C, H, W) for PyTorch
    data = load_dataset()
    data_t = torch.tensor(data).permute(0, 3, 1, 2)  # NCHW

    dataset = TensorDataset(data_t)
    loader = DataLoader(dataset, batch_size=batch_size, shuffle=True)

    model = build_autoencoder(latent_dim=32).to(device)
    optimizer = torch.optim.Adam(model.parameters(), lr=lr)
    criterion = nn.MSELoss()

    start_epoch = 0
    if resume and (MODEL_DIR / "checkpoint.pt").exists():
        ckpt = torch.load(MODEL_DIR / "checkpoint.pt", map_location=device, weights_only=True)
        model.load_state_dict(ckpt["model"])
        optimizer.load_state_dict(ckpt["optimizer"])
        start_epoch = ckpt["epoch"]
        print(f"Resumed from epoch {start_epoch} (loss={ckpt['loss']:.6f})")
    elif resume and (MODEL_DIR / "autoencoder.pt").exists():
        model.load_state_dict(torch.load(MODEL_DIR / "autoencoder.pt", map_location=device, weights_only=True))
        print("Resumed from saved autoencoder.pt (optimizer state reset)")

    print(f"\nTraining: {len(data)} samples, epochs {start_epoch+1}-{start_epoch+epochs}, batch_size={batch_size}")
    print(f"Model params: {sum(p.numel() for p in model.parameters()):,}")
    print()

    best_loss = float("inf")
    total_epochs = start_epoch + epochs
    for epoch in range(start_epoch + 1, total_epochs + 1):
        model.train()
        total_loss = 0.0
        for (batch,) in loader:
            batch = batch.to(device)
            recon, z = model(batch)
            loss = criterion(recon, batch)
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
            total_loss += loss.item() * batch.size(0)

        avg_loss = total_loss / len(data)
        if (epoch - start_epoch) % 10 == 0 or epoch == start_epoch + 1:
            print(f"  Epoch {epoch:4d}/{total_epochs}  loss={avg_loss:.6f}")

        if avg_loss < best_loss:
            best_loss = avg_loss

    # Save model + checkpoint
    MODEL_DIR.mkdir(parents=True, exist_ok=True)
    torch.save(model.state_dict(), MODEL_DIR / "autoencoder.pt")
    torch.save(model.encoder.state_dict(), MODEL_DIR / "encoder.pt")
    torch.save({
        "epoch": total_epochs,
        "model": model.state_dict(),
        "optimizer": optimizer.state_dict(),
        "loss": best_loss,
    }, MODEL_DIR / "checkpoint.pt")

    # Save latent vectors for all tiles
    model.eval()
    with torch.no_grad():
        all_data = data_t.to(device)
        _, latents = model(all_data)
        latents_np = latents.cpu().numpy()
        np.save(MODEL_DIR / "latent_vectors.npy", latents_np)

    # Save sample reconstructions
    model.eval()
    with torch.no_grad():
        sample = data_t[:8].to(device)
        recon, _ = model(sample)
        recon = recon.cpu().permute(0, 2, 3, 1).numpy()
        sample_np = data_t[:8].permute(0, 2, 3, 1).numpy()

        # Stitch originals and reconstructions side by side
        rows = []
        for i in range(min(8, len(sample_np))):
            row = np.concatenate([sample_np[i], recon[i]], axis=1)
            rows.append(row)
        grid = np.concatenate(rows, axis=0)
        grid = (np.clip(grid, 0, 1) * 255).astype(np.uint8)
        Image.fromarray(grid).save(MODEL_DIR / "reconstructions.png")

    print(f"\nDone! Best loss: {best_loss:.6f}")
    print(f"Model saved:        {MODEL_DIR / 'autoencoder.pt'}")
    print(f"Encoder saved:      {MODEL_DIR / 'encoder.pt'}")
    print(f"Latent vectors:     {MODEL_DIR / 'latent_vectors.npy'}")
    print(f"Reconstructions:    {MODEL_DIR / 'reconstructions.png'}")


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main():
    parser = argparse.ArgumentParser(description="EotB tile autoencoder training pipeline")
    parser.add_argument("--slice-only", action="store_true", help="Only slice tiles, don't train")
    parser.add_argument("--train-only", action="store_true", help="Skip slicing, train on existing data")
    parser.add_argument("--epochs", type=int, default=100, help="Training epochs (default: 100)")
    parser.add_argument("--lr", type=float, default=1e-3, help="Learning rate (default: 1e-3)")
    parser.add_argument("--batch-size", type=int, default=16, help="Batch size (default: 16)")
    parser.add_argument("--resume", action="store_true", help="Resume training from last checkpoint")
    args = parser.parse_args()

    if not args.train_only:
        print("=" * 60)
        print("Step 1: Slicing sprite sheets")
        print("=" * 60)
        entries = run_slicing()

        print("\n" + "=" * 60)
        print("Step 2: Augmenting dataset")
        print("=" * 60)
        augment_tiles(entries)

    if not args.slice_only:
        print("\n" + "=" * 60)
        print("Step 3: Training autoencoder")
        print("=" * 60)
        train(epochs=args.epochs, lr=args.lr, batch_size=args.batch_size, resume=args.resume)


if __name__ == "__main__":
    main()
