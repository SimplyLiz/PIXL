---
title: "Wave Function Collapse: Technical Implementation"
source: "https://www.gridbugs.org/wave-function-collapse/"
topic: "procedural-generation"
fetched: "2026-03-23"
---

# Wave Function Collapse: Technical Implementation

Based on gridbugs.org detailed implementation guide

## Core Algorithm Architecture

WFC comprises three main components:

### 1. Image Preprocessing

Extracts tiles from input images and generates adjacency rules + frequency hints. Given a tile size, the preprocessor enumerates all tile-sized squares from the input (including wrapped edges). Each unique tile receives an index, and occurrence counts become frequency hints.

### 2. Core Algorithm

The central constraint-satisfaction engine. Maintains a probability distribution for each output cell and repeatedly collapses cells while propagating constraints.

### 3. Image Postprocessing

Converts the tile-indexed grid into a final image by mapping each index to its corresponding color/sprite.

## Data Structures

```
CoreCell:
  - possible: Vec<bool>                         # which tiles may appear
  - sum_of_possible_tile_weights: usize
  - sum_of_possible_tile_weight_log_weights: f32
  - entropy_noise: f32                           # tie-breaking noise
  - is_collapsed: bool
  - tile_enabler_counts: Vec<TileEnablerCount>

TileEnablerCount:
  - by_direction: [usize; 4]  # enablers per cardinal direction

CoreState:
  - grid: Grid2D<CoreCell>
  - adjacency_rules: AdjacencyRules
  - frequency_hints: FrequencyHints
  - entropy_heap: BinaryHeap<EntropyCoord>       # min-entropy priority queue
  - tile_removals: Vec<RemovalUpdate>             # propagation stack
```

## Entropy Calculation

Uses information-theoretic entropy to select which cell to collapse next:

```
entropy = log(W) - (sum of w * log(w)) / W
```

Where W = sum of all possible tile weights, w = individual tile weight.

The entropy is cached and incrementally updated as tiles are removed, giving O(1) per lookup via the binary heap.

## Cell Collapse Process

1. **Select minimum-entropy cell** from the binary heap (ties broken with pre-computed noise)
2. **Choose tile probabilistically** weighted by frequency hints
3. **Remove all other possibilities** from the selected cell
4. **Queue removal updates** for constraint propagation

## Constraint Propagation (Enabler Counting)

The key insight: "A tile may not appear in a cell unless it has at least one compatible tile in every cardinal direction."

### Algorithm

1. Pop removal update from stack (tile T removed from cell C)
2. For each direction D:
   - Find neighboring cell N in direction D
   - For each tile T' that was compatible with T in direction D:
     - Decrement T' 's enabler count for the opposite direction
     - If enabler count reaches zero → remove T' from cell N
     - Queue new removal, update entropy heap
3. Repeat until stack is empty

### Enabler Count Initialization

For each cell, for each possible tile, for each direction: count how many compatible tiles exist in the adjacency rules for that direction.

## Adjacency Rules Generation

For tiles of size T×T:

```
compatible(a, b, direction) -> bool:
    Offset tile b by 1 pixel in the given direction
    Check if ALL overlapping pixels match between a and b
    Return true only if all overlapping regions are identical
```

## Contradiction Handling

When a cell loses all possible tiles, the generation has failed. Options:
- **Restart**: Abandon and regenerate (simple, most common)
- **Backtrack**: Checkpoint state, undo recent collapses, try alternatives (complex but more robust)

## Weighted Tile Selection

```
sum = total weight of all possible tiles in cell
r = random_integer(0, sum)
for each possible tile:
    r -= tile.weight
    if r < 0: return tile
```

## Rotation and Reflection

The preprocessor can generate 8 variants per extracted tile (4 rotations × 2 reflections). Each variant becomes a distinct tile with its own index and adjacency rules. This greatly increases variety from small input samples.

## References

- Paul Merrell's 2007 "Model Synthesis" — the academic precursor
- Maxim Gumin's canonical implementation: github.com/mxgmn/WaveFunctionCollapse
- fast-wfc (C++ optimized): github.com/math-fehr/fast-wfc
