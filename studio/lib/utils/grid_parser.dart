/// Extract a PAX grid block from Claude's response text.
/// Looks for content between ``` markers or a raw grid block.
String? extractGrid(String response) {
  // Try to find a code block
  final codeBlockRegex = RegExp(r'```(?:\w*\n)?([\s\S]*?)```');
  final match = codeBlockRegex.firstMatch(response);
  if (match != null) {
    final block = match.group(1)!.trim();
    final lines = block.split('\n').where((l) => l.trim().isNotEmpty).toList();
    if (lines.length >= 4 && lines.every((l) => l.length >= 4)) {
      return block;
    }
  }

  // Try to find raw grid lines (consecutive lines of similar-length symbol chars)
  final lines = response.split('\n');
  final gridLines = <String>[];
  for (final line in lines) {
    final trimmed = line.trim();
    // Skip markdown: headers (# Title), bullets (* item, - item).
    // Lines like "####" are valid grid rows, not markdown headers.
    final isMarkdown = (trimmed.startsWith('* ') ||
        trimmed.startsWith('- ') ||
        (trimmed.startsWith('#') && trimmed.contains(' ')));
    if (trimmed.isNotEmpty &&
        trimmed.length >= 4 &&
        !isMarkdown &&
        !trimmed.contains(' ')) {
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
