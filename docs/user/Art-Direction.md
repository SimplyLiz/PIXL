# Art Direction & Quality

PIXL reviews your sprites automatically and tells you exactly what to fix — not a vague score, but specific notes with row numbers.

## What gets checked

Every tile runs through structural validators:

| Check | What it means | When it flags |
|-------|--------------|---------------|
| **Outline coverage** | % of silhouette edges with dark border pixels | Below 70% = fix, below 30% = redo |
| **Centering** | Is the subject in the middle of the tile? | Off-center by more than 30% |
| **Canvas utilization** | How much of the tile does the subject fill? | Under 40% = too small |
| **Contrast** | Can you tell adjacent parts apart? | Very low contrast = muddy |
| **Fragmentation** | Are there disconnected floating pixels? | More than 3 separate regions |

## The refine loop

1. **Create** a tile (draw it, generate it, or import it)
2. **Critique** — PIXL analyzes the rendered result and reports issues
3. **Fix** — patch the specific rows that were flagged
4. **Re-check** — PIXL confirms the fix or finds remaining issues
5. Repeat until it passes (usually 2-3 rounds)

Each round is fast because you're fixing targeted problems, not starting over.

## Fix instructions

PIXL doesn't just say "outline is weak." It says:

> Only 67% of boundary pixels are dark — outline is incomplete. Fill gaps in the silhouette border. Outline gaps found at rows: 3, 7, 12.

Specific enough to act on immediately.

## Severity levels

- **Error** — the tile should be regenerated. Too broken to fix with patches.
- **Warning** — fixable. Use the refine tool to patch specific rows.
- **Info** — noting a metric, no action needed.

## Works on everything

The quality system runs the same checks whether you painted the tile by hand, generated it with AI, or imported it from Aseprite. Same standards, same feedback.

## CLI usage

```bash
pixl critique tileset.pax --tile wizard
```

Output:
```
pixl critique: 'wizard' (16x16)

  Outline coverage:    67.0%
  Centering:           91.0%
  Canvas utilization:  66.0%
  Mean contrast:       0.274

  ! Only 67% of boundary pixels are dark — outline is incomplete.

  Verdict: REFINE — fix the issues above.
```
