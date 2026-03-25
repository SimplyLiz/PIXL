/// Extract a PAX grid block from an LLM response.
/// Handles multiple formats:
///   1. PAX TOML: grid = """...""" blocks
///   2. Code-fenced: ```...``` blocks
///   3. Raw grid: consecutive lines of symbol characters
String? extractGrid(String response) {
  // 1. Try PAX TOML grid = """...""" blocks (take the first one)
  final paxGridRegex = RegExp(r'grid\s*=\s*"""([\s\S]*?)"""');
  final paxMatch = paxGridRegex.firstMatch(response);
  if (paxMatch != null) {
    final block = paxMatch.group(1)!.trim();
    final lines = block.split('\n').where((l) => l.trim().isNotEmpty).toList();
    if (lines.length >= 4 && lines.every((l) => l.trim().length >= 4)) {
      return lines.map((l) => l.trim()).join('\n');
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
      final lines = inner.split('\n').where((l) => l.trim().isNotEmpty).toList();
      if (lines.length >= 4) {
        return lines.map((l) => l.trim()).join('\n');
      }
    }
    // Otherwise treat the whole block as a grid if it looks like one
    final lines = block.split('\n').where((l) => l.trim().isNotEmpty).toList();
    if (lines.length >= 4 && lines.every((l) => l.trim().length >= 4 && !l.contains(' = '))) {
      return block;
    }
  }

  // 3. Try raw grid lines (consecutive non-space lines of similar length)
  final lines = response.split('\n');
  final gridLines = <String>[];
  for (final line in lines) {
    final trimmed = line.trim();
    // Skip TOML config lines, markdown, and blank lines
    if (trimmed.contains(' = ') || trimmed.contains('=') && trimmed.contains('"')) continue;
    final isMarkdown = (trimmed.startsWith('* ') ||
        trimmed.startsWith('- ') ||
        (trimmed.startsWith('#') && trimmed.contains(' ')));
    if (trimmed.isNotEmpty &&
        trimmed.length >= 4 &&
        !isMarkdown &&
        !trimmed.contains(' ') &&
        !trimmed.startsWith('[') &&
        !trimmed.startsWith('"""')) {
      gridLines.add(trimmed);
    } else if (gridLines.isNotEmpty) {
      break;
    }
  }
  if (gridLines.length >= 4) {
    return gridLines.join('\n');
  }

  return null;
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
