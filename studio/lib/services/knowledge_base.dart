/// Composes system prompts from backend-retrieved knowledge and session style.
///
/// Knowledge retrieval (BM25 + knowledge graph) happens server-side in
/// pixl serve via /api/generate/context. This class only assembles the
/// pieces the backend returns with the active style fragment.
class KnowledgeBase {
  /// Build a system prompt from backend context and active style.
  ///
  /// [backendContext] is the system_prompt returned by /api/generate/context,
  /// which already contains targeted knowledge passages from the BM25 corpus.
  static String buildSystemPrompt({
    String? backendContext,
    String? styleFragment,
  }) {
    final parts = <String>[];

    if (backendContext != null && backendContext.isNotEmpty) {
      parts.add(backendContext);
    }
    if (styleFragment != null && styleFragment.isNotEmpty) {
      parts.add('\n## Active Style\n$styleFragment');
    }

    return parts.join('\n');
  }
}
