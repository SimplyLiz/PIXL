// Backtracking extension for WFC.
//
// Current implementation uses retry-based recovery (restart with seed+1).
// Snapshot-based backtracking (save/restore cell states) is a V1.1 optimization
// for complex tilesets that need finer-grained recovery than full restarts.
//
// The retry approach is simpler and sufficient for V1:
// - WFC runs with seed N
// - On contradiction: retry with seed N+1 (up to max_retries)
// - Different seed = different collapse order = different result
// - For most tilesets (>12 tiles), 5 retries is enough for >90% success
