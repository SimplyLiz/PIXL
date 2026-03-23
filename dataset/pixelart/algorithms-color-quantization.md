---
title: "Color Quantization Algorithms"
source: "https://en.wikipedia.org/wiki/Color_quantization"
topic: "algorithms"
fetched: "2026-03-23"
---

# Color Quantization Algorithms

## Overview

Color quantization reduces the number of distinct colors in an image while preserving visual appearance. Essential for pixel art workflows: importing reference images, converting photographs to limited palettes, and extracting palettes from existing art.

The process has two phases:
1. **Palette design**: Select a small set of representative colors
2. **Pixel mapping**: Assign each input pixel to the nearest palette color

## Median Cut

Invented by Paul Heckbert in 1979. The most widely-used algorithm.

### Algorithm

1. Place all image pixels into a single box in RGB color space
2. Find the box with the largest range along any color axis (R, G, or B)
3. Sort pixels in that box along the largest-range axis
4. Split the box at the median pixel into two boxes
5. Repeat steps 2–4 until the desired number of boxes (= palette size) is reached
6. Each box's final color = mean of all pixels in that box

### Properties

- **Pros**: Good color distribution, handles common colors well, deterministic
- **Cons**: Can split important color clusters, uniform box shapes miss color space geometry
- **Complexity**: O(n log n) per split (sorting), O(k) splits total

## Octree Quantization

Conceived by Gervautz and Purgathofer, improved by Dan Bloomberg at Xerox PARC.

### Algorithm

1. Initialize an octree with root representing entire RGB space
2. For each pixel, traverse the tree from root:
   - At each level, use one bit each from R, G, B to select the child octant
   - Increment the leaf node's pixel count and color sum
3. When the tree exceeds the desired leaf count:
   - Find the deepest leaf with the fewest pixels
   - Merge it into its parent (combine color sums and counts)
4. Repeat until leaf count = desired palette size
5. Each leaf's final color = color sum / pixel count

### Properties

- **Pros**: Naturally spatial, handles large images efficiently, progressive refinement
- **Cons**: 8-way branching can be memory-heavy, merge order affects results
- **Complexity**: O(n) to build tree, O(n) to reduce

## K-Means Clustering

### Algorithm

1. Choose k initial cluster centers (random pixels, or seeded from another method)
2. Assign each pixel to the nearest cluster center (Euclidean distance in RGB)
3. Recalculate each cluster center as the mean of its assigned pixels
4. Repeat steps 2–3 until centers converge (stop moving) or max iterations reached
5. Final centers = palette colors

### Properties

- **Pros**: Optimal for minimizing total color distance, natural clustering
- **Cons**: Sensitive to initialization, may converge to local optima, non-deterministic
- **Complexity**: O(n × k × iterations)

### Variants

- **K-Means++**: Smarter initialization (spread-out initial centers) → better results
- **Mini-batch K-Means**: Process random subsets per iteration → faster for large images

## Dithering During Quantization

After palette selection, error diffusion distributes quantization error to neighboring pixels:

### Floyd-Steinberg Dithering

For each pixel, left to right, top to bottom:
1. Find the nearest palette color
2. Calculate the error (original − quantized)
3. Distribute error to neighbors:
   - Right: 7/16
   - Below-left: 3/16
   - Below: 5/16
   - Below-right: 1/16

### Ordered Dithering

Apply a threshold matrix (Bayer matrix) to add structured noise before quantization. Creates regular, visible patterns instead of diffused error.

### Atkinson Dithering

Similar to Floyd-Steinberg but distributes only 3/4 of the error (the rest is discarded). Creates higher contrast with more visible dither patterns. Popular for its Apple Macintosh aesthetic.

## Practical Considerations for Pixel Art

- **Median cut** works well for extracting palettes from reference images
- **K-means** is best when you want to find the "true" N most important colors
- **Octree** is fastest for real-time or very large images
- **For pixel art conversion**: Quantize first, then hand-edit the palette. No algorithm perfectly captures artistic intent.
- **Perceptual color spaces** (CIELAB/OKLab) give better results than RGB for distance calculations — colors that look similar are numerically close
