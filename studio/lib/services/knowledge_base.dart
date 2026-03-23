import 'package:flutter/services.dart';

/// Loads the curated pixel art knowledge base from bundled assets.
/// This is injected as the system prompt prefix for all Claude interactions.
class KnowledgeBase {
  static String? _cached;

  /// Load the knowledge base. Caches after first load.
  static Future<String> load() async {
    _cached ??= await rootBundle.loadString('assets/knowledge_base.md');
    return _cached!;
  }

  /// Build a complete system prompt combining the knowledge base
  /// with session-specific context (theme, palette, constraints).
  static Future<String> buildSystemPrompt({
    String? backendContext,
    String? styleFragment,
  }) async {
    final kb = await load();
    final parts = <String>[kb];

    if (backendContext != null && backendContext.isNotEmpty) {
      parts.add('\n## Current Session Context\n$backendContext');
    }
    if (styleFragment != null && styleFragment.isNotEmpty) {
      parts.add('\n## Active Style\n$styleFragment');
    }

    return parts.join('\n');
  }
}
