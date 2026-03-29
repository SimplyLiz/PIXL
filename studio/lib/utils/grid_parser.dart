/// Extract a PAX grid block from an LLM response.
/// Handles multiple formats:
///   1. PAX TOML: grid = """...""" blocks
///   2. Code-fenced: ```...``` blocks
///   3. Raw grid: consecutive lines of symbol characters
///   4. Longest consistent block: finds the largest group of similar-width lines
String? extractGrid(String response) {
  // 1. Try PAX TOML grid = """...""" blocks (take the first one)
  final paxGridRegex = RegExp(r'grid\s*=\s*"""([\s\S]*?)"""');
  final paxMatch = paxGridRegex.firstMatch(response);
  if (paxMatch != null) {
    final block = paxMatch.group(1)!.trim();
    final lines = block.split('\n').where((l) => l.trim().isNotEmpty).map((l) => l.trim()).toList();
    if (lines.length >= 2) {
      return _normalizeRowWidths(lines).join('\n');
    }
  }

  // 2. Try code-fenced blocks
  final codeBlockRegex = RegExp(r'```(?:\w*\n)?([\s\S]*?)```');
  final codeMatch = codeBlockRegex.firstMatch(response);
  if (codeMatch != null) {
    final block = codeMatch.group(1)!.trim();
    // Check if the code block itself contains a PAX grid = """..."""
    final innerPax = paxGridRegex.firstMatch(block);
    if (innerPax != null) {
      final inner = innerPax.group(1)!.trim();
      final lines = inner.split('\n').where((l) => l.trim().isNotEmpty).map((l) => l.trim()).toList();
      if (lines.length >= 2) {
        return _normalizeRowWidths(lines).join('\n');
      }
    }
    // Otherwise treat the whole block as a grid if it looks like one
    final lines = block.split('\n').where((l) => l.trim().isNotEmpty).map((l) => l.trim()).toList();
    if (lines.length >= 2 && lines.every((l) => !l.contains(' = ') && l.trim().length >= 2)) {
      return _normalizeRowWidths(lines).join('\n');
    }
  }

  // 3. Find the longest run of similar-width symbol-only lines
  final allLines = response.split('\n');
  final candidates = <(int, int, int)>[]; // (startIndex, length, lineWidth)

  int i = 0;
  while (i < allLines.length) {
    final trimmed = allLines[i].trim();

    // Skip non-grid lines
    if (!_isGridLine(trimmed)) {
      i++;
      continue;
    }

    // Found a potential grid line — collect consecutive similar-width lines
    final width = trimmed.length;
    final start = i;
    final gridLines = <String>[trimmed];
    i++;

    while (i < allLines.length) {
      final next = allLines[i].trim();
      if (!_isGridLine(next)) break;
      // Allow some width tolerance (within 2 chars) for ragged model output
      if ((next.length - width).abs() <= 2) {
        gridLines.add(next);
        i++;
      } else {
        break;
      }
    }

    if (gridLines.length >= 2) {
      candidates.add((start, gridLines.length, width));
    }
  }

  if (candidates.isNotEmpty) {
    // Pick the longest run (most lines), break ties by widest
    candidates.sort((a, b) {
      final cmp = b.$2.compareTo(a.$2); // most lines first
      if (cmp != 0) return cmp;
      return b.$3.compareTo(a.$3); // widest first
    });
    final (startIdx, count, _) = candidates.first;
    final result = <String>[];
    for (var j = startIdx; j < startIdx + count; j++) {
      result.add(allLines[j].trim());
    }
    return _normalizeRowWidths(result).join('\n');
  }

  return null;
}

/// Normalize all rows to the same width (the most common width).
/// Truncates longer rows from the right, discards rows that are too short.
List<String> _normalizeRowWidths(List<String> rows) {
  if (rows.isEmpty) return rows;
  // Find the most common width (mode)
  final widthCounts = <int, int>{};
  for (final r in rows) {
    widthCounts[r.length] = (widthCounts[r.length] ?? 0) + 1;
  }
  final targetWidth = widthCounts.entries
      .reduce((a, b) => a.value >= b.value ? a : b)
      .key;
  final result = <String>[];
  for (final r in rows) {
    if (r.length == targetWidth) {
      result.add(r);
    } else if (r.length > targetWidth) {
      // Truncate trailing extra symbols
      result.add(r.substring(0, targetWidth));
    }
    // Skip rows that are too short — likely malformed
  }
  return result;
}

/// Check if a line looks like a grid row (symbol characters, no prose).
bool _isGridLine(String trimmed) {
  if (trimmed.isEmpty || trimmed.length < 2) return false;
  // Skip TOML config lines
  if (trimmed.contains(' = ') || (trimmed.contains('=') && trimmed.contains('"'))) return false;
  // Skip markdown headers with text (but "####" alone IS a valid grid row)
  if (trimmed.startsWith('#') && trimmed.contains(' ')) return false;
  // Skip bullet lists
  if (trimmed.startsWith('* ') || trimmed.startsWith('- ')) return false;
  // Skip lines with spaces (prose), brackets (TOML), triple quotes
  if (trimmed.contains(' ') || trimmed.startsWith('[') || trimmed.startsWith('"""')) return false;
  // Must be symbol characters only
  return true;
}

/// Ensure [name] is unique among [existingNames], appending _2, _3, etc.
String uniqueTileName(String name, Set<String> existingNames) {
  if (!existingNames.contains(name)) return name;
  for (var i = 2;; i++) {
    final candidate = '${name}_$i';
    if (!existingNames.contains(candidate)) return candidate;
  }
}

/// Generate a tile name from a user prompt.
String generateTileName(String prompt) {
  final words = prompt
      .toLowerCase()
      .replaceAll(RegExp(r'[^a-z0-9\s]'), '')
      .split(RegExp(r'\s+'))
      .where((w) => !{
            'generate', 'create', 'make', 'draw', 'me', 'a', 'an', 'the',
            'tile', 'pixel',
          }.contains(w))
      .take(3)
      .toList();
  if (words.isEmpty) words.add('tile');
  final name = words.join('_');
  final suffix = DateTime.now().millisecondsSinceEpoch % 10000;
  return '${name}_$suffix';
}
