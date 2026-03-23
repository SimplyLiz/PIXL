import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/backend_provider.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/chat_provider.dart';
import '../../theme/studio_theme.dart';

/// Left panel — AI expert chat with generation context bridge.
class ChatPanel extends ConsumerStatefulWidget {
  const ChatPanel({super.key});

  @override
  ConsumerState<ChatPanel> createState() => _ChatPanelState();
}

class _ChatPanelState extends ConsumerState<ChatPanel> {
  final _controller = TextEditingController();
  final _scrollController = ScrollController();
  bool _generating = false;

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

    final backend = ref.read(backendProvider);
    final isGenRequest = _isGenerationRequest(text);

    if (backend.isConnected && isGenRequest) {
      await _handleGenerationRequest(text);
    } else if (backend.isConnected) {
      await _handleGeneralMessage(text);
    } else {
      chat.addAssistantMessage(
        'Engine not connected. Start `pixl serve` or click reconnect in the Engine panel.',
      );
    }
    _scrollToBottom();
  }

  bool _isGenerationRequest(String text) {
    final lower = text.toLowerCase();
    return lower.contains('generate') ||
        lower.contains('create') ||
        lower.contains('make me') ||
        lower.contains('draw me') ||
        lower.startsWith('a ') && lower.contains('tile');
  }

  Future<void> _handleGenerationRequest(String prompt) async {
    setState(() => _generating = true);
    final chat = ref.read(chatProvider.notifier);
    final canvasSize = ref.read(canvasProvider).canvasSize;
    final sizeStr = '${canvasSize.width}x${canvasSize.height}';

    chat.addAssistantMessage('Getting generation context from engine...');
    _scrollToBottom();

    final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
      prompt: prompt,
      size: sizeStr,
    );

    if (ctx.containsKey('error')) {
      chat.addAssistantMessage('Engine error: ${ctx['error']}');
    } else {
      final systemPrompt = ctx['system_prompt'] as String? ?? '';
      final userPrompt = ctx['user_prompt'] as String? ?? '';
      final palette = ctx['palette'] as String? ?? '';
      final theme = ctx['theme'] as String? ?? '';

      final response = StringBuffer();
      response.writeln('**Generation context ready**\n');

      if (theme.isNotEmpty) {
        response.writeln('Theme: `$theme`');
      }
      if (palette.isNotEmpty) {
        response.writeln('Palette: `$palette`');
      }
      response.writeln('Size: `$sizeStr`\n');
      response.writeln('---\n');
      response.writeln('**Enriched prompt** (send this to Claude):\n');

      if (userPrompt.isNotEmpty) {
        response.writeln('```\n$userPrompt\n```\n');
      } else {
        response.writeln('```\n$prompt\n```\n');
      }

      if (systemPrompt.isNotEmpty) {
        response.writeln('<details>\n<summary>System context (${systemPrompt.length} chars)</summary>\n');
        response.writeln('```\n${systemPrompt.length > 500 ? '${systemPrompt.substring(0, 500)}...' : systemPrompt}\n```\n');
        response.writeln('</details>');
      }

      response.writeln('\n*Copy the enriched prompt above to generate with Claude, or connect the Anthropic API key in settings.*');

      chat.addAssistantMessage(response.toString());
    }

    setState(() => _generating = false);
  }

  Future<void> _handleGeneralMessage(String text) async {
    final chat = ref.read(chatProvider.notifier);
    final lower = text.toLowerCase();

    // Handle some local commands the backend can answer
    if (lower.contains('validate') || lower.contains('check')) {
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
          msg.writeln('- (warning) $warn');
        }
        chat.addAssistantMessage(msg.toString());
      }
    } else if (lower.contains('tiles') || lower.contains('list')) {
      await ref.read(backendProvider.notifier).refreshTiles();
      final tiles = ref.read(backendProvider).tiles;
      if (tiles.isEmpty) {
        chat.addAssistantMessage('No tiles in the current session.');
      } else {
        final msg = StringBuffer('**${tiles.length} tiles:**\n');
        for (final t in tiles) {
          msg.writeln('- `${t.name}` ${t.size ?? ''}');
        }
        chat.addAssistantMessage(msg.toString());
      }
    } else if (lower.contains('export') || lower.contains('pax')) {
      final source = await ref.read(backendProvider.notifier).getPaxSource();
      if (source != null) {
        chat.addAssistantMessage('**PAX source** (${source.length} bytes):\n\n```toml\n${source.length > 800 ? '${source.substring(0, 800)}...' : source}\n```');
      } else {
        chat.addAssistantMessage('Could not retrieve PAX source.');
      }
    } else {
      chat.addAssistantMessage(
        'I can help with:\n'
        '- **"generate a wall tile"** — get enriched prompts\n'
        '- **"validate"** — check edge compat & palette\n'
        '- **"list tiles"** — show session tiles\n'
        '- **"export pax"** — show PAX source\n\n'
        'AI chat with Claude API is coming in Phase 3.',
      );
    }
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
                // Connection indicator
                Container(
                  width: 6, height: 6,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: backend.isConnected
                        ? const Color(0xFF4caf50)
                        : const Color(0xFF888888),
                  ),
                ),
                const SizedBox(width: 8),
                _IconBtn(
                  icon: Icons.delete_outline,
                  onTap: () => ref.read(chatProvider.notifier).clear(),
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
                    enabled: !_generating,
                    style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                    maxLines: 3,
                    minLines: 1,
                    decoration: InputDecoration(
                      hintText: backend.isConnected
                          ? 'Ask or generate...'
                          : 'Engine offline...',
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
                _generating
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
