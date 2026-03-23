import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/backend_provider.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/chat_provider.dart';
import '../../providers/claude_provider.dart';
import '../../services/claude_api.dart';
import '../../theme/studio_theme.dart';

/// Left panel — AI expert chat with Claude generation pipeline.
///
/// Flow:
///   1. User types prompt (e.g. "generate a 16x16 dungeon wall tile")
///   2. Chat calls /api/generate/context → enriched system + user prompt
///   3. Sends to Claude API → gets PAX grid response
///   4. Calls /api/tile/create with the grid → validates + renders
///   5. Shows preview → user accepts or requests variations
class ChatPanel extends ConsumerStatefulWidget {
  const ChatPanel({super.key});

  @override
  ConsumerState<ChatPanel> createState() => _ChatPanelState();
}

class _ChatPanelState extends ConsumerState<ChatPanel> {
  final _controller = TextEditingController();
  final _scrollController = ScrollController();

  // Pending generated tile for accept/reject flow
  String? _pendingTileName;
  String? _pendingPreviewB64;
  String? _lastGenerationPrompt;

  void _scrollToBottom() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 200),
          curve: Curves.easeOut,
        );
      }
    });
  }

  Future<void> _send() async {
    final text = _controller.text.trim();
    if (text.isEmpty) return;

    final chat = ref.read(chatProvider.notifier);
    chat.addUserMessage(text);
    _controller.clear();
    _scrollToBottom();

    if (_isGenerationRequest(text)) {
      await _handleGeneration(text);
    } else {
      await _handleChat(text);
    }
    _scrollToBottom();
  }

  bool _isGenerationRequest(String text) {
    final lower = text.toLowerCase();
    return lower.contains('generate') ||
        lower.contains('create a') ||
        lower.contains('create me') ||
        lower.contains('make a') ||
        lower.contains('make me') ||
        lower.contains('draw a') ||
        lower.contains('draw me');
  }

  // ── Generation Flow ──────────────────────────────────────

  Future<void> _handleGeneration(String prompt) async {
    final chat = ref.read(chatProvider.notifier);
    final backend = ref.read(backendProvider);
    final claude = ref.read(claudeProvider);
    final canvasSize = ref.read(canvasProvider).canvasSize;
    final sizeStr = '${canvasSize.width}x${canvasSize.height}';

    _lastGenerationPrompt = prompt;

    // Step 1: Check prerequisites
    if (!backend.isConnected) {
      chat.addAssistantMessage(
        'Engine not connected. Start `pixl serve` or reconnect in the Engine panel.',
      );
      return;
    }
    if (!claude.hasApiKey) {
      chat.addAssistantMessage(
        'No API key configured. Click **Settings** in the top bar to add your Anthropic API key.',
      );
      return;
    }

    // Step 2: Get enriched context from backend
    chat.addAssistantMessage('Getting generation context...');
    _scrollToBottom();

    final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
      prompt: prompt,
      size: sizeStr,
    );

    if (ctx.containsKey('error')) {
      chat.addAssistantMessage('Engine error: ${ctx['error']}');
      return;
    }

    final systemPrompt = ctx['system_prompt'] as String? ?? '';
    final userPrompt = ctx['user_prompt'] as String? ?? prompt;
    final themeName = ctx['theme'] as String? ?? '';

    // Step 3: Call Claude API
    chat.addAssistantMessage(
      'Generating $sizeStr tile with **${claude.model.split('-').take(2).join(' ')}**...'
      '${themeName.isNotEmpty ? ' (theme: $themeName)' : ''}',
    );
    _scrollToBottom();

    final resp = await ref.read(claudeProvider.notifier).generateTile(
      systemPrompt: systemPrompt,
      userPrompt: userPrompt,
    );

    if (resp.isError) {
      chat.addAssistantMessage('Generation failed: ${resp.errorMessage}');
      return;
    }

    // Step 4: Extract PAX grid from response
    final grid = _extractGrid(resp.content);
    if (grid == null) {
      chat.addAssistantMessage(
        'Claude responded but I couldn\'t extract a valid grid.\n\n'
        '**Response:**\n${resp.content}\n\n'
        '*${resp.totalTokens} tokens used*',
      );
      return;
    }

    // Step 5: Create tile via backend → validate + render
    final tileName = _generateTileName(prompt);
    final createResp = await ref.read(backendProvider.notifier).createTile(
      name: tileName,
      palette: ctx['palette'] as String? ?? 'default',
      size: sizeStr,
      grid: grid,
    );

    if (createResp.containsKey('error')) {
      chat.addAssistantMessage(
        'Tile validation failed: ${createResp['error']}\n\n'
        '**Generated grid:**\n```\n$grid\n```',
      );
      return;
    }

    // Step 6: Get preview
    final previewB64 = createResp['preview'] as String?;

    setState(() {
      _pendingTileName = tileName;
      _pendingPreviewB64 = previewB64;

    });

    final validationInfo = createResp['validation'] as Map<String, dynamic>?;
    final isValid = validationInfo?['valid'] as bool? ?? true;

    chat.addAssistantMessage(
      '**Generated: `$tileName`** ($sizeStr)\n\n'
      '${isValid ? 'Validation passed.' : 'Validation warnings — check the validation panel.'}\n\n'
      '*${resp.totalTokens} tokens*\n\n'
      'Use the buttons below to **accept** or **reject**, or request **variations**.',
    );
    _scrollToBottom();
  }

  /// Accept the pending tile — it stays in the session.
  void _acceptTile() {
    final chat = ref.read(chatProvider.notifier);
    if (_pendingTileName != null) {
      chat.addAssistantMessage('Accepted **`$_pendingTileName`**. Tile is in the session.');
      ref.read(backendProvider.notifier).refreshTiles();
    }
    setState(() {
      _pendingTileName = null;
      _pendingPreviewB64 = null;

    });
    _scrollToBottom();
  }

  /// Reject the pending tile — delete from session.
  void _rejectTile() {
    final chat = ref.read(chatProvider.notifier);
    if (_pendingTileName != null) {
      ref.read(backendProvider.notifier).deleteTile(_pendingTileName!);
      chat.addAssistantMessage('Rejected **`$_pendingTileName`**. Tile removed.');
    }
    setState(() {
      _pendingTileName = null;
      _pendingPreviewB64 = null;

    });
    _scrollToBottom();
  }

  /// Re-generate with the same prompt (variation).
  Future<void> _generateVariation() async {
    // Delete current pending tile first
    if (_pendingTileName != null) {
      ref.read(backendProvider.notifier).deleteTile(_pendingTileName!);
    }
    setState(() {
      _pendingTileName = null;
      _pendingPreviewB64 = null;

    });

    if (_lastGenerationPrompt != null) {
      await _handleGeneration(_lastGenerationPrompt!);
    }
  }

  // ── Chat Flow ────────────────────────────────────────────

  Future<void> _handleChat(String text) async {
    final chat = ref.read(chatProvider.notifier);
    final backend = ref.read(backendProvider);
    final claude = ref.read(claudeProvider);
    final lower = text.toLowerCase();

    // Local commands that don't need Claude
    if (lower.contains('validate') || lower.contains('check')) {
      if (!backend.isConnected) {
        chat.addAssistantMessage('Engine not connected.');
        return;
      }
      chat.addAssistantMessage('Running validation...');
      _scrollToBottom();
      final report = await ref.read(backendProvider.notifier).validate(checkEdges: true);
      if (report.valid) {
        chat.addAssistantMessage('All checks passed.');
      } else {
        final msg = StringBuffer('**Validation issues:**\n');
        for (final err in report.errors) {
          msg.writeln('- $err');
        }
        for (final warn in report.warnings) {
          msg.writeln('- (warn) $warn');
        }
        chat.addAssistantMessage(msg.toString());
      }
      return;
    }

    if (lower.contains('list tiles') || lower == 'tiles') {
      if (!backend.isConnected) {
        chat.addAssistantMessage('Engine not connected.');
        return;
      }
      await ref.read(backendProvider.notifier).refreshTiles();
      final tiles = ref.read(backendProvider).tiles;
      if (tiles.isEmpty) {
        chat.addAssistantMessage('No tiles in session.');
      } else {
        final msg = StringBuffer('**${tiles.length} tiles:**\n');
        for (final t in tiles) {
          msg.writeln('- `${t.name}` ${t.size ?? ''}');
        }
        chat.addAssistantMessage(msg.toString());
      }
      return;
    }

    if (lower.contains('export') || lower.contains('pax source')) {
      if (!backend.isConnected) {
        chat.addAssistantMessage('Engine not connected.');
        return;
      }
      final source = await ref.read(backendProvider.notifier).getPaxSource();
      if (source != null) {
        chat.addAssistantMessage(
          '**PAX source** (${source.length} bytes):\n\n'
          '```toml\n${source.length > 1200 ? '${source.substring(0, 1200)}...' : source}\n```',
        );
      } else {
        chat.addAssistantMessage('Could not retrieve PAX source.');
      }
      return;
    }

    // Send to Claude as expert chat if API key is configured
    if (claude.hasApiKey && backend.isConnected) {
      // Get context for chat
      final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
        prompt: text,
        type: 'chat',
      );
      final systemPrompt = ctx['system_prompt'] as String? ??
          'You are a pixel art expert. Help the user with pixel art techniques, '
          'palette design, tiling, and the PAX format.';

      // Build message history from chat
      final messages = ref.read(chatProvider)
          .where((m) => m.role == 'user' || m.role == 'assistant')
          .map((m) => ClaudeMessage(role: m.role, content: m.content))
          .toList();

      // Keep last 10 messages for context
      final recentMessages = messages.length > 10
          ? messages.sublist(messages.length - 10)
          : messages;

      final resp = await ref.read(claudeProvider.notifier).chat(
        systemPrompt: systemPrompt,
        messages: recentMessages,
      );

      if (resp.isError) {
        chat.addAssistantMessage('Error: ${resp.errorMessage}');
      } else {
        chat.addAssistantMessage(resp.content);
      }
    } else if (!claude.hasApiKey) {
      chat.addAssistantMessage(
        'Add your Anthropic API key in **Settings** to chat with the AI expert.\n\n'
        'Without the API key, I can handle:\n'
        '- **"validate"** — check edge compat & palette\n'
        '- **"list tiles"** — show session tiles\n'
        '- **"export pax"** — show PAX source',
      );
    } else {
      chat.addAssistantMessage('Engine not connected. Start `pixl serve` first.');
    }
  }

  // ── Helpers ──────────────────────────────────────────────

  /// Extract a grid block from Claude's response.
  /// Looks for content between ``` markers or a raw grid block.
  String? _extractGrid(String response) {
    // Try to find a code block
    final codeBlockRegex = RegExp(r'```(?:\w*\n)?([\s\S]*?)```');
    final match = codeBlockRegex.firstMatch(response);
    if (match != null) {
      final block = match.group(1)!.trim();
      // Verify it looks like a grid (lines of similar length with symbols)
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
      if (trimmed.isNotEmpty &&
          trimmed.length >= 4 &&
          !trimmed.startsWith('*') &&
          !trimmed.startsWith('#') &&
          !trimmed.startsWith('-') &&
          !trimmed.contains(' ')) {
        gridLines.add(trimmed);
      } else if (gridLines.isNotEmpty) {
        break; // End of grid block
      }
    }
    if (gridLines.length >= 4) {
      return gridLines.join('\n');
    }

    return null;
  }

  /// Generate a tile name from the prompt.
  String _generateTileName(String prompt) {
    final words = prompt.toLowerCase()
        .replaceAll(RegExp(r'[^a-z0-9\s]'), '')
        .split(RegExp(r'\s+'))
        .where((w) => !{'generate', 'create', 'make', 'draw', 'me', 'a', 'an', 'the', 'tile', 'pixel'}.contains(w))
        .take(3)
        .toList();
    if (words.isEmpty) words.add('tile');
    final name = words.join('_');
    // Add timestamp suffix to avoid collisions
    final suffix = DateTime.now().millisecondsSinceEpoch % 10000;
    return '${name}_$suffix';
  }

  @override
  void dispose() {
    _controller.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final messages = ref.watch(chatProvider);
    final theme = Theme.of(context);
    final backend = ref.watch(backendProvider);
    final claude = ref.watch(claudeProvider);
    final isGenerating = claude.isGenerating;

    return Container(
      width: 260,
      decoration: StudioTheme.panelDecoration,
      child: Column(
        children: [
          // Header
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
            child: Row(
              children: [
                Icon(Icons.auto_awesome, size: 14, color: theme.colorScheme.primary),
                const SizedBox(width: 6),
                Text('AI EXPERT', style: theme.textTheme.titleSmall),
                const Spacer(),
                // Connection indicators
                _StatusDot(
                  color: backend.isConnected ? const Color(0xFF4caf50) : const Color(0xFF888888),
                  tooltip: backend.isConnected ? 'Engine connected' : 'Engine offline',
                ),
                const SizedBox(width: 4),
                _StatusDot(
                  color: claude.hasApiKey ? const Color(0xFF4caf50) : const Color(0xFFffaa00),
                  tooltip: claude.hasApiKey ? 'API key set' : 'No API key',
                ),
                const SizedBox(width: 8),
                _IconBtn(
                  icon: Icons.delete_outline,
                  onTap: () {
                    ref.read(chatProvider.notifier).clear();
                    setState(() {
                      _pendingTileName = null;
                      _pendingPreviewB64 = null;
                
                    });
                  },
                  tooltip: 'Clear chat',
                ),
              ],
            ),
          ),
          const Divider(height: 1),

          // Messages
          Expanded(
            child: ListView.builder(
              controller: _scrollController,
              padding: const EdgeInsets.all(8),
              itemCount: messages.length,
              itemBuilder: (context, index) {
                final msg = messages[index];
                final isUser = msg.role == 'user';
                return Padding(
                  padding: const EdgeInsets.only(bottom: 8),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text(
                        isUser ? 'You' : 'PIXL',
                        style: theme.textTheme.bodySmall!.copyWith(
                          color: isUser ? theme.colorScheme.secondary : theme.colorScheme.primary,
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                      const SizedBox(height: 2),
                      MarkdownBody(
                        data: msg.content,
                        styleSheet: MarkdownStyleSheet(
                          p: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                          code: theme.textTheme.bodyMedium!.copyWith(
                            fontSize: 11,
                            backgroundColor: const Color(0xFF2a2a4e),
                          ),
                          codeblockDecoration: BoxDecoration(
                            color: const Color(0xFF2a2a4e),
                            borderRadius: BorderRadius.circular(4),
                          ),
                        ),
                      ),
                    ],
                  ),
                );
              },
            ),
          ),

          // Pending tile accept/reject/variation bar
          if (_pendingTileName != null)
            Container(
              padding: const EdgeInsets.all(8),
              decoration: const BoxDecoration(
                border: Border(top: StudioTheme.panelBorder),
                color: Color(0xFF1a1a30),
              ),
              child: Column(
                children: [
                  // Preview
                  if (_pendingPreviewB64 != null)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: ClipRRect(
                        borderRadius: BorderRadius.circular(4),
                        child: Image.memory(
                          base64Decode(_pendingPreviewB64!),
                          width: 120, height: 120,
                          filterQuality: FilterQuality.none,
                          fit: BoxFit.contain,
                        ),
                      ),
                    ),
                  Text(
                    _pendingTileName!,
                    style: theme.textTheme.bodySmall!.copyWith(fontSize: 10),
                    overflow: TextOverflow.ellipsis,
                  ),
                  const SizedBox(height: 6),
                  Row(
                    children: [
                      Expanded(
                        child: _ActionButton(
                          label: 'Accept',
                          color: const Color(0xFF4caf50),
                          onTap: _acceptTile,
                        ),
                      ),
                      const SizedBox(width: 4),
                      Expanded(
                        child: _ActionButton(
                          label: 'Reject',
                          color: const Color(0xFFf44336),
                          onTap: _rejectTile,
                        ),
                      ),
                      const SizedBox(width: 4),
                      Expanded(
                        child: _ActionButton(
                          label: 'Vary',
                          color: theme.colorScheme.primary,
                          onTap: isGenerating ? null : _generateVariation,
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),

          // Input
          Container(
            padding: const EdgeInsets.all(8),
            decoration: const BoxDecoration(
              border: Border(top: StudioTheme.panelBorder),
            ),
            child: Row(
              children: [
                Expanded(
                  child: TextField(
                    controller: _controller,
                    enabled: !isGenerating,
                    style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                    maxLines: 3,
                    minLines: 1,
                    decoration: InputDecoration(
                      hintText: isGenerating
                          ? 'Generating...'
                          : claude.hasApiKey
                              ? 'Ask or "generate a wall tile"...'
                              : 'Add API key in Settings...',
                      hintStyle: theme.textTheme.bodySmall,
                      isDense: true,
                      contentPadding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(4),
                        borderSide: StudioTheme.panelBorder,
                      ),
                      focusedBorder: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(4),
                        borderSide: BorderSide(color: theme.colorScheme.primary),
                      ),
                    ),
                    onSubmitted: (_) => _send(),
                  ),
                ),
                const SizedBox(width: 4),
                isGenerating
                    ? const SizedBox(
                        width: 24, height: 24,
                        child: CircularProgressIndicator(strokeWidth: 1.5),
                      )
                    : _IconBtn(icon: Icons.send, onTap: _send, tooltip: 'Send'),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _StatusDot extends StatelessWidget {
  const _StatusDot({required this.color, required this.tooltip});
  final Color color;
  final String tooltip;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: Container(
        width: 6, height: 6,
        decoration: BoxDecoration(shape: BoxShape.circle, color: color),
      ),
    );
  }
}

class _IconBtn extends StatelessWidget {
  const _IconBtn({required this.icon, required this.onTap, this.tooltip});
  final IconData icon;
  final VoidCallback onTap;
  final String? tooltip;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip ?? '',
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(4),
        child: Padding(
          padding: const EdgeInsets.all(4),
          child: Icon(icon, size: 16),
        ),
      ),
    );
  }
}

class _ActionButton extends StatelessWidget {
  const _ActionButton({required this.label, required this.color, this.onTap});
  final String label;
  final Color color;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(4),
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 6),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(4),
          border: Border.all(color: onTap != null ? color : color.withValues(alpha: 0.3)),
        ),
        child: Center(
          child: Text(
            label,
            style: TextStyle(
              fontSize: 10,
              fontWeight: FontWeight.w700,
              color: onTap != null ? color : color.withValues(alpha: 0.3),
            ),
          ),
        ),
      ),
    );
  }
}
