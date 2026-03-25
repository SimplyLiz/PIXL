import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/backend_provider.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/chat_provider.dart';
import '../../providers/claude_provider.dart';
import '../../providers/style_provider.dart';
import '../../services/llm_provider.dart';
import '../../services/knowledge_base.dart';
import '../../theme/studio_theme.dart';
import '../../utils/grid_parser.dart';

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

  // Variation system — multiple alternatives shown side by side
  List<_TileVariation> _variations = [];
  int _selectedVariation = -1;
  bool _isGeneratingVariations = false;

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
        'No API key configured. Click **Settings** in the top bar to configure a provider.',
      );
      return;
    }

    // ── Local LoRA path — server-side generation ──
    if (claude.provider == LlmProviderType.pixlLocal) {
      await _handleLocalGeneration(prompt, sizeStr);
      return;
    }

    // ── Cloud LLM path — client-side generation ──

    // Step 2: Get enriched context from backend
    chat.addAssistantMessage('Getting generation context...', isStatus: true);
    _scrollToBottom();

    final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
      prompt: prompt,
      size: sizeStr,
    );

    if (ctx.containsKey('error')) {
      chat.addAssistantMessage('Engine error: ${ctx['error']}');
      return;
    }

    final backendContext = ctx['system_prompt'] as String? ?? '';
    final userPrompt = ctx['user_prompt'] as String? ?? prompt;
    final themeName = ctx['theme'] as String? ?? '';

    // Step 3: Build enriched system prompt with knowledge base + style
    final style = ref.read(styleProvider);
    final systemPrompt = await KnowledgeBase.buildSystemPrompt(
      backendContext: backendContext,
      styleFragment: style.toPromptFragment(),
    );

    chat.addAssistantMessage(
      'Generating $sizeStr tile with **${claude.model.split('-').take(2).join(' ')}**...'
      '${themeName.isNotEmpty ? ' (theme: $themeName)' : ''}',
      isStatus: true,
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

    // Step 4: Check if the response is a full PAX source (multiple tiles)
    final content = resp.content;
    if (_isPaxSource(content)) {
      // Extract PAX source from code fence or raw
      final paxSource = _extractPaxSource(content);
      chat.addAssistantMessage('Loading generated tileset...', isStatus: true);
      _scrollToBottom();

      final loadResp = await ref.read(backendProvider.notifier).loadSource(paxSource);
      if (loadResp.containsKey('error')) {
        chat.addAssistantMessage(
          'Failed to load generated PAX: ${loadResp['error']}\n\n'
          '*${resp.totalTokens} tokens used*',
        );
        return;
      }

      final tiles = ref.read(backendProvider).tiles;
      chat.addAssistantMessage(
        '**Loaded tileset** (${tiles.length} tiles)\n\n'
        '*${resp.totalTokens} tokens*',
      );
      _scrollToBottom();
      return;
    }

    // Step 4b: Extract single grid from response
    final grid = extractGrid(content);
    if (grid == null) {
      chat.addAssistantMessage(
        'The model responded but I couldn\'t extract a valid grid.\n\n'
        '**Response:**\n${content.length > 500 ? '${content.substring(0, 500)}...' : content}\n\n'
        '*${resp.totalTokens} tokens used*',
      );
      return;
    }

    // Step 5: Create tile via backend → validate + render
    final tileName = generateTileName(prompt);
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

  // ── Local LoRA Generation ─────────────────────────────────
  // Entire pipeline runs server-side: context building, inference, grid
  // extraction, and tile creation — all in a single /api/generate/tile call.

  Future<void> _handleLocalGeneration(String prompt, String sizeStr) async {
    final chat = ref.read(chatProvider.notifier);
    final tileName = generateTileName(prompt);

    chat.addAssistantMessage(
      'Generating $sizeStr tile with **PIXL LoRA** (on-device)...',
      isStatus: true,
    );
    _scrollToBottom();

    final resp = await ref.read(backendProvider.notifier).generateTile(
      name: tileName,
      prompt: prompt,
      size: sizeStr,
    );

    if (resp['ok'] != true) {
      final error = resp['error'] as String? ?? 'Unknown error';
      final hint = resp['hint'] as String?;
      chat.addAssistantMessage(
        'Generation failed: $error'
        '${hint != null ? '\n\n**Hint:** `$hint`' : ''}',
      );
      return;
    }

    final previewB64 = resp['preview_b64'] as String?;

    setState(() {
      _pendingTileName = tileName;
      _pendingPreviewB64 = previewB64;
    });

    final generated = resp['generated'] == true;
    chat.addAssistantMessage(
      '**Generated: `$tileName`** ($sizeStr)\n\n'
      '${generated ? 'Generated on-device with LoRA adapter.' : 'Created.'}\n\n'
      'Use the buttons below to **accept** or **reject**, or request **variations**.',
    );
    _scrollToBottom();
  }

  /// Check if the response looks like a full PAX TOML source.
  bool _isPaxSource(String content) {
    // Look for PAX structure markers
    return content.contains('[theme]') ||
        content.contains('[[tiles]]') ||
        content.contains('[palettes.') ||
        (content.contains('[pax]') && content.contains('version'));
  }

  /// Extract PAX source from a response — handles code fences.
  String _extractPaxSource(String content) {
    // Try code-fenced block first
    final fenceRegex = RegExp(r'```(?:toml|pax)?\n([\s\S]*?)```');
    final match = fenceRegex.firstMatch(content);
    if (match != null) {
      final block = match.group(1)!.trim();
      if (_isPaxSource(block)) return block;
    }

    // Strip any leading/trailing prose — find first [ and last ]
    final firstBracket = content.indexOf('[');
    if (firstBracket >= 0) {
      return content.substring(firstBracket).trim();
    }
    return content.trim();
  }

  /// Accept the pending tile — record feedback, update style latent.
  Future<void> _acceptTile() async {
    final chat = ref.read(chatProvider.notifier);
    if (_pendingTileName != null) {
      // Record feedback — triggers style latent update in engine
      final resp = await ref.read(backendProvider.notifier).backend.recordFeedback(
        name: _pendingTileName!,
        action: 'accept',
      );
      final rate = resp['acceptance_rate'];
      final rateStr = rate != null ? ' (${(rate * 100).round()}% acceptance rate)' : '';
      final autoLearn = ref.read(claudeProvider).autoLearn;
      final learnStr = autoLearn ? ' Saved as training data.' : '';
      chat.addAssistantMessage('Accepted **`$_pendingTileName`**.$rateStr$learnStr');
      ref.read(backendProvider.notifier).refreshTiles();
    }
    setState(() {
      _pendingTileName = null;
      _pendingPreviewB64 = null;
    });
    _scrollToBottom();
  }

  /// Reject the pending tile — show reason selector, record feedback.
  void _rejectTile() {
    if (_pendingTileName == null) return;
    _showRejectReasonPicker();
  }

  void _showRejectReasonPicker() {
    final reasons = {
      'too_sparse': 'Too sparse',
      'too_dense': 'Too dense',
      'wrong_style': 'Wrong style',
      'bad_edges': 'Bad edges',
      'palette_violation': 'Palette issue',
      'bad_composition': 'Bad composition',
    };

    showDialog(
      context: context,
      builder: (ctx) => SimpleDialog(
        title: const Text('Why reject?', style: TextStyle(fontSize: 14)),
        children: [
          ...reasons.entries.map((e) => SimpleDialogOption(
            onPressed: () {
              Navigator.of(ctx).pop();
              _doReject(e.key);
            },
            child: Text(e.value, style: const TextStyle(fontSize: 12)),
          )),
          SimpleDialogOption(
            onPressed: () {
              Navigator.of(ctx).pop();
              _doReject(null);
            },
            child: const Text('Skip (no reason)', style: TextStyle(fontSize: 12, fontStyle: FontStyle.italic)),
          ),
        ],
      ),
    );
  }

  Future<void> _doReject(String? reason) async {
    final chat = ref.read(chatProvider.notifier);
    if (_pendingTileName != null) {
      // Record feedback with reason
      await ref.read(backendProvider.notifier).backend.recordFeedback(
        name: _pendingTileName!,
        action: 'reject',
        rejectReason: reason,
      );
      ref.read(backendProvider.notifier).deleteTile(_pendingTileName!);
      final reasonText = reason != null ? ' (${reason.replaceAll('_', ' ')})' : '';
      chat.addAssistantMessage('Rejected **`$_pendingTileName`**$reasonText');
    }
    setState(() {
      _pendingTileName = null;
      _pendingPreviewB64 = null;
    });
    _scrollToBottom();
  }

  /// Re-generate with the same prompt (single variation).
  Future<void> _generateVariation() async {
    // Record reject + delete current pending tile
    if (_pendingTileName != null) {
      await ref.read(backendProvider.notifier).backend.recordFeedback(
        name: _pendingTileName!,
        action: 'reject',
        rejectReason: 'wrong_style',
      );
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

  /// Generate 3 variations in parallel, show as selectable strip.
  Future<void> _generateVariations() async {
    if (_lastGenerationPrompt == null) return;
    final prompt = _lastGenerationPrompt!;

    // Clean up current pending tile
    if (_pendingTileName != null) {
      await ref.read(backendProvider.notifier).backend.recordFeedback(
        name: _pendingTileName!,
        action: 'reject',
        rejectReason: 'wrong_style',
      );
      ref.read(backendProvider.notifier).deleteTile(_pendingTileName!);
    }

    setState(() {
      _pendingTileName = null;
      _pendingPreviewB64 = null;
      _variations = [];
      _selectedVariation = -1;
      _isGeneratingVariations = true;
    });

    final chat = ref.read(chatProvider.notifier);
    final backend = ref.read(backendProvider);
    final claude = ref.read(claudeProvider);
    final canvasSize = ref.read(canvasProvider).canvasSize;
    final sizeStr = '${canvasSize.width}x${canvasSize.height}';

    if (!backend.isConnected || !claude.hasApiKey) return;

    // Local LoRA path — generate sequentially (server handles one at a time)
    if (claude.provider == LlmProviderType.pixlLocal) {
      chat.addAssistantMessage('Generating 3 variations with **PIXL LoRA**...', isStatus: true);
      _scrollToBottom();

      final results = <_TileVariation>[];
      for (var i = 0; i < 3; i++) {
        final tileName = '${generateTileName(prompt)}_v${i + 1}';
        final resp = await ref.read(backendProvider.notifier).generateTile(
          name: tileName,
          prompt: prompt,
          size: sizeStr,
        );
        if (resp['ok'] == true) {
          results.add(_TileVariation(
            name: tileName,
            previewB64: resp['preview_b64'] as String?,
          ));
        }
        if (mounted) setState(() => _variations = List.of(results));
        _scrollToBottom();
      }
      if (mounted) setState(() => _isGeneratingVariations = false);
      if (results.isNotEmpty) {
        chat.addAssistantMessage(
          '**${results.length} variations** generated. Pick the one you like.',
        );
      }
      _scrollToBottom();
      return;
    }

    // Cloud LLM path — 3 parallel calls
    chat.addAssistantMessage(
      'Generating 3 variations with **${claude.model.split('-').take(2).join(' ')}**...',
      isStatus: true,
    );
    _scrollToBottom();

    final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
      prompt: prompt,
      size: sizeStr,
    );
    if (ctx.containsKey('error')) {
      chat.addAssistantMessage('Engine error: ${ctx['error']}');
      setState(() => _isGeneratingVariations = false);
      return;
    }

    final backendContext = ctx['system_prompt'] as String? ?? '';
    final userPrompt = ctx['user_prompt'] as String? ?? prompt;
    final style = ref.read(styleProvider);
    final systemPrompt = await KnowledgeBase.buildSystemPrompt(
      backendContext: backendContext,
      styleFragment: style.toPromptFragment(),
    );

    // Fire 3 generation calls in parallel
    final futures = List.generate(3, (i) async {
      final resp = await ref.read(claudeProvider.notifier).service.generate(
        systemPrompt: systemPrompt,
        userPrompt: userPrompt,
        temperature: 0.5 + (i * 0.15), // slight temperature variation
      );
      if (resp.isError) return null;

      final grid = extractGrid(resp.content);
      if (grid == null) return null;

      final tileName = '${generateTileName(prompt)}_v${i + 1}';
      final createResp = await ref.read(backendProvider.notifier).createTile(
        name: tileName,
        palette: ctx['palette'] as String? ?? 'default',
        size: sizeStr,
        grid: grid,
      );
      if (createResp.containsKey('error')) return null;

      return _TileVariation(
        name: tileName,
        previewB64: createResp['preview'] as String?,
      );
    });

    final results = await Future.wait(futures);
    final valid = results.whereType<_TileVariation>().toList();

    if (mounted) {
      setState(() {
        _variations = valid;
        _isGeneratingVariations = false;
      });
    }

    if (valid.isEmpty) {
      chat.addAssistantMessage('All 3 variations failed. Try a simpler prompt or different model.');
    } else {
      chat.addAssistantMessage(
        '**${valid.length} variation${valid.length > 1 ? 's' : ''}** generated. Pick the one you like.',
      );
    }
    _scrollToBottom();
  }

  /// Accept one variation, delete the rest.
  Future<void> _acceptVariation(int index) async {
    final chat = ref.read(chatProvider.notifier);
    final pick = _variations[index];

    // Record accept for chosen
    final resp = await ref.read(backendProvider.notifier).backend.recordFeedback(
      name: pick.name,
      action: 'accept',
    );
    final rate = resp['acceptance_rate'];
    final rateStr = rate != null ? ' (${(rate * 100).round()}% acceptance rate)' : '';

    // Delete unchosen variations
    for (var i = 0; i < _variations.length; i++) {
      if (i != index) {
        await ref.read(backendProvider.notifier).backend.recordFeedback(
          name: _variations[i].name,
          action: 'reject',
          rejectReason: 'variation_not_chosen',
        );
        ref.read(backendProvider.notifier).deleteTile(_variations[i].name);
      }
    }

    chat.addAssistantMessage('Accepted **`${pick.name}`**.$rateStr');
    ref.read(backendProvider.notifier).refreshTiles();

    setState(() {
      _variations = [];
      _selectedVariation = -1;
    });
    _scrollToBottom();
  }

  /// Reject all variations.
  Future<void> _rejectAllVariations() async {
    final chat = ref.read(chatProvider.notifier);
    for (final v in _variations) {
      await ref.read(backendProvider.notifier).backend.recordFeedback(
        name: v.name,
        action: 'reject',
        rejectReason: 'variation_not_chosen',
      );
      ref.read(backendProvider.notifier).deleteTile(v.name);
    }
    chat.addAssistantMessage('Rejected all variations.');
    setState(() {
      _variations = [];
      _selectedVariation = -1;
    });
    _scrollToBottom();
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
      chat.addAssistantMessage('Running validation...', isStatus: true);
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
      // Build system prompt with knowledge base + session context
      final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
        prompt: text,
        type: 'chat',
      );
      final backendCtx = ctx['system_prompt'] as String? ?? '';
      final style = ref.read(styleProvider);
      final systemPrompt = await KnowledgeBase.buildSystemPrompt(
        backendContext: backendCtx,
        styleFragment: style.toPromptFragment(),
      );

      // Build message history from chat
      // Filter out status messages — they waste tokens in Claude's context.
      final messages = ref.read(chatProvider)
          .where((m) => !m.isStatus && (m.role == 'user' || m.role == 'assistant'))
          .map((m) => LlmMessage(role: m.role, content: m.content))
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

  // Helpers moved to lib/utils/grid_parser.dart

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
                  color: backend.isConnected ? StudioTheme.success : StudioTheme.separatorColor,
                  tooltip: backend.isConnected ? 'Engine connected' : 'Engine offline',
                ),
                const SizedBox(width: 4),
                _StatusDot(
                  color: claude.hasApiKey ? StudioTheme.success : StudioTheme.warning,
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
                      _lastGenerationPrompt = null;
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
                            backgroundColor: StudioTheme.codeBg,
                          ),
                          codeblockDecoration: BoxDecoration(
                            color: StudioTheme.codeBg,
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

          // Variation strip — multiple alternatives to pick from
          if (_variations.isNotEmpty || _isGeneratingVariations)
            Container(
              padding: const EdgeInsets.all(8),
              decoration: const BoxDecoration(
                border: Border(top: StudioTheme.panelBorder),
                color: StudioTheme.recessedBg,
              ),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Row(
                    children: [
                      Text('VARIATIONS', style: theme.textTheme.titleSmall),
                      if (_isGeneratingVariations) ...[
                        const SizedBox(width: 8),
                        const SizedBox(
                          width: 10, height: 10,
                          child: CircularProgressIndicator(strokeWidth: 1.5),
                        ),
                      ],
                    ],
                  ),
                  const SizedBox(height: 8),
                  Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    children: [
                      for (var i = 0; i < _variations.length; i++) ...[
                        if (i > 0) const SizedBox(width: 6),
                        GestureDetector(
                          onTap: () => setState(() => _selectedVariation = i),
                          child: Container(
                            decoration: BoxDecoration(
                              borderRadius: BorderRadius.circular(4),
                              border: Border.all(
                                color: _selectedVariation == i
                                    ? theme.colorScheme.primary
                                    : theme.dividerColor,
                                width: _selectedVariation == i ? 2 : 1,
                              ),
                            ),
                            child: ClipRRect(
                              borderRadius: BorderRadius.circular(3),
                              child: _variations[i].previewB64 != null
                                  ? Image.memory(
                                      base64Decode(_variations[i].previewB64!),
                                      width: 64, height: 64,
                                      filterQuality: FilterQuality.none,
                                      fit: BoxFit.contain,
                                    )
                                  : Container(
                                      width: 64, height: 64,
                                      color: StudioTheme.codeBg,
                                      child: const Center(
                                        child: Icon(Icons.image_not_supported, size: 16),
                                      ),
                                    ),
                            ),
                          ),
                        ),
                      ],
                      // Placeholder slots while generating
                      for (var i = _variations.length; i < 3 && _isGeneratingVariations; i++) ...[
                        if (i > 0 || _variations.isNotEmpty) const SizedBox(width: 6),
                        Container(
                          width: 64, height: 64,
                          decoration: BoxDecoration(
                            borderRadius: BorderRadius.circular(4),
                            border: Border.all(color: theme.dividerColor),
                            color: StudioTheme.codeBg,
                          ),
                          child: const Center(
                            child: SizedBox(
                              width: 14, height: 14,
                              child: CircularProgressIndicator(strokeWidth: 1.5),
                            ),
                          ),
                        ),
                      ],
                    ],
                  ),
                  if (_selectedVariation >= 0 && _selectedVariation < _variations.length) ...[
                    const SizedBox(height: 4),
                    Center(
                      child: Text(
                        _variations[_selectedVariation].name,
                        style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
                        overflow: TextOverflow.ellipsis,
                      ),
                    ),
                  ],
                  const SizedBox(height: 8),
                  if (!_isGeneratingVariations && _variations.isNotEmpty)
                    Row(
                      children: [
                        Expanded(
                          child: _ActionButton(
                            label: 'Accept',
                            color: StudioTheme.success,
                            onTap: _selectedVariation >= 0
                                ? () => _acceptVariation(_selectedVariation)
                                : null,
                          ),
                        ),
                        const SizedBox(width: 4),
                        Expanded(
                          child: _ActionButton(
                            label: 'Reject All',
                            color: StudioTheme.error,
                            onTap: _rejectAllVariations,
                          ),
                        ),
                      ],
                    ),
                ],
              ),
            ),

          // Pending tile accept/reject/variation bar (single tile)
          if (_pendingTileName != null && _variations.isEmpty)
            Container(
              padding: const EdgeInsets.all(8),
              decoration: const BoxDecoration(
                border: Border(top: StudioTheme.panelBorder),
                color: StudioTheme.recessedBg,
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
                          color: StudioTheme.success,
                          onTap: _acceptTile,
                        ),
                      ),
                      const SizedBox(width: 4),
                      Expanded(
                        child: _ActionButton(
                          label: 'Reject',
                          color: StudioTheme.error,
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
                  const SizedBox(height: 4),
                  _ActionButton(
                    label: 'Variations (3)',
                    color: theme.colorScheme.secondary,
                    onTap: isGenerating ? null : _generateVariations,
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

class _TileVariation {
  const _TileVariation({required this.name, this.previewB64});
  final String name;
  final String? previewB64;
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
