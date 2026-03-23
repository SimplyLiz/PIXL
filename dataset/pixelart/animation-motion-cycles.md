---
title: "Motion Cycles in Pixel Art"
source: "https://www.slynyrd.com/blog/2020/1/23/pixelblog-25-motion-cycles"
topic: "animation"
fetched: "2026-03-23"
---

# Motion Cycles in Pixel Art

By Raymond Schlitter (SLYNYRD)

## Human Walk Cycle

Walk cycles are more challenging than run cycles due to their subtle movements.

### Approach

- Start by making a figure with body parts clearly defined by color
- Animate sequentially: legs first, then arms, head, and body movement
- Keep character designs simple to maintain clean movement
- Study real people to add personality based on age, body type, and circumstances

### Frame Structure

1. **Contact**: Front foot touches ground, rear foot pushes off
2. **Down**: Body at lowest point, weight transfers
3. **Passing**: Rear leg swings forward past stance leg
4. **Up**: Body at highest point, front leg extends
5. **Contact (mirror)**: Opposite foot now forward

Minimum viable: 4 frames (2 unique + 2 mirrored). Sweet spot: 6–8 frames.

## Quadrupedal Walk (Dog)

- Master bipedal cycles before attempting quadrupeds
- Treat four legs as two sets of biped pairs (front pair, rear pair)
- Ensure consistent distance traveled per frame to prevent accordion-like stretching
- Monitor undulating motion of back pelvis and front shoulders for balanced rhythm
- The rear pair leads slightly in phase, creating a diagonal gait pattern

## Bird Flight

- Use 8 frames to show the complete wing motion arc
- Mark wing joints clearly to prevent "rubbery" appearance
- Animate front view before attempting side views
- Wing downstroke is faster/more powerful than upstroke
- Add body bob: body rises on downstroke, drops on upstroke
- Final details (feather separation, tail movement) added after establishing basic motion

## General Principles

- Animation requires a lot of repetitive practice on a regular basis
- Start simple, add complexity only after the base motion reads well
- Energy and readability matter more than frame count
- Always preview at target game speed, not just in the editor timeline
- Each cycle should loop seamlessly — the last frame must transition smoothly into the first
