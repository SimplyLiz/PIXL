import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_markdown/flutter_markdown.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/backend_provider.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/chat_provider.dart';
import '../../models/palette.dart';
import '../../providers/palette_provider.dart';
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
  late final _focusNode = FocusNode(
    onKeyEvent: (node, event) {
      if (event is! KeyDownEvent) return KeyEventResult.ignored;
      final key = event.logicalKey;
      final isMeta = HardwareKeyboard.instance.isMetaPressed ||
                     HardwareKeyboard.instance.isControlPressed;

      if (key == LogicalKeyboardKey.enter) {
        if (isMeta) {
          // Cmd+Enter → insert newline
          final sel = _controller.selection;
          final text = _controller.text;
          _controller.value = TextEditingValue(
            text: '${text.substring(0, sel.baseOffset)}\n${text.substring(sel.baseOffset)}',
            selection: TextSelection.collapsed(offset: sel.baseOffset + 1),
          );
          return KeyEventResult.handled;
        }
        // Enter → send
        _send();
        return KeyEventResult.handled;
      }

      if (key == LogicalKeyboardKey.arrowUp &&
          _controller.selection.baseOffset == 0) {
        _historyUp();
        return KeyEventResult.handled;
      }
      if (key == LogicalKeyboardKey.arrowDown &&
          _controller.selection.baseOffset == _controller.text.length) {
        _historyDown();
        return KeyEventResult.handled;
      }
      return KeyEventResult.ignored;
    },
  );

  // Input history (up/down arrow recall)
  final _inputHistory = <String>[];
  int _historyIndex = -1;
  String _savedInput = '';

  // Pending generated tile for accept/reject flow
  String? _pendingTileName;

  String? _lastGenerationPrompt;

  // Variation system — multiple alternatives shown side by side
  List<_TileVariation> _variations = [];
  int _selectedVariation = -1;
  bool _isGeneratingVariations = false;

  // Click-to-reference: selected tile for next message context
  TileInfo? _referencedTile;

  // Track generating state for typing indicator scroll
  bool _wasGenerating = false;

  @override
  void initState() {
    super.initState();
    _focusNode.addListener(() {
      if (mounted) setState(() {});
    });
  }

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

    // Capture and clear referenced tile before sending.
    final refTile = _referencedTile;
    setState(() => _referencedTile = null);

    // Track input history
    _inputHistory.add(text);
    _historyIndex = -1;
    _savedInput = '';

    final chat = ref.read(chatProvider.notifier);
    chat.addUserMessage(text);
    _controller.clear();
    _scrollToBottom();

    if (_isAiCommand(text)) {
      await _handleAiCommand(text);
    } else if (_isGenerationRequest(text)) {
      await _handleGeneration(text, refTile: refTile);
    } else {
      await _handleChat(text, refTile: refTile);
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

  /// Ensure the engine has the active palette registered and return its name.
  ///
  /// The engine's palette key (e.g. 'dungeon') differs from both the theme ID
  /// ('dark_fantasy') and the display name ('Dark Fantasy'). If no PAX file is
  /// loaded yet, bootstraps via /api/new → /api/load with the full template.
  Future<String?> _ensurePalette() async {
    final palette = ref.read(paletteProvider);
    final enginePalette = palette.enginePalette ?? 'dungeon';
    final engineId = palette.engineId ?? 'dark_fantasy';
    final backend = ref.read(backendProvider.notifier);

    // Check if the palette already exists in the engine.
    // getPalette takes a theme name (e.g. 'dark_fantasy'), not a palette name.
    final check = await backend.backend.getPalette(engineId);
    if (!check.containsKey('error')) return enginePalette;

    // Palette missing — bootstrap full template via /api/new → /api/load.
    final tmpl = await backend.backend.newFromTemplate(engineId);
    final source = tmpl['source'] as String?;
    if (source != null && source.isNotEmpty) {
      final loadResp = await backend.loadSource(source);
      if (!loadResp.containsKey('error')) {
        // Sync palette provider from the freshly loaded template.
        final synced = PixlPalette.fromEngineResponse(loadResp);
        if (synced != null) {
          ref.read(paletteProvider.notifier).setPalette(synced);
          return synced.enginePalette ?? enginePalette;
        }
        return enginePalette;
      }
      ref.read(chatProvider.notifier).addAssistantMessage(
        'Failed to load template: ${loadResp['error']}',
      );
      return null;
    }

    // /api/new failed — tell user to load manually.
    ref.read(chatProvider.notifier).addAssistantMessage(
      'No palette loaded. Create a **New Project** or **Open a .pax file** from the top bar.',
    );
    return null;
  }

  // ── Generation Flow ──────────────────────────────────────

  Future<void> _handleGeneration(String prompt, {TileInfo? refTile}) async {
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

    // Step 2: Ensure the engine has a palette loaded before anything else.
    final ensuredPalette = await _ensurePalette();
    if (ensuredPalette == null) return;

    // Step 3: Get enriched context from backend
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
    var userPrompt = ctx['user_prompt'] as String? ?? prompt;
    final themeName = ctx['theme'] as String? ?? '';

    // Inject referenced tile context if present.
    if (refTile != null) {
      final edges = refTile.edgeClasses?.entries.map((e) => '${e.key}=${e.value}').join(', ') ?? 'unknown';
      final tags = refTile.tags.isNotEmpty ? refTile.tags.join(', ') : 'none';
      userPrompt = '[Reference tile: ${refTile.name}, size: ${refTile.size ?? 'unknown'}, '
          'edges: $edges, tags: $tags]\n\n$userPrompt';
    }

    // Step 3: Build enriched system prompt with knowledge base + style
    final style = ref.read(styleProvider);
    final systemPrompt = KnowledgeBase.buildSystemPrompt(
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
    var grid = extractGrid(content);

    // Note: grid dimensions may not match canvas size (small models struggle
    // with exact size constraints). We accept the actual dimensions rather than
    // rejecting or retrying — the engine handles arbitrary tile sizes.

    if (grid == null) {
      final modelName = claude.model;
      final isSmall = modelName.contains('llama3.2') || modelName.contains('phi') ||
          modelName.contains('gemma') || modelName.contains('3b') || modelName.contains('3B');
      final hint = isSmall
          ? '\n\n**Hint:** `$modelName` is a small model that struggles with structured grid output. '
            'Try **qwen3:8b**, **llama3.1:8b**, or a cloud provider (Claude, GPT-4o) for reliable PAX generation.'
          : '';
      // Show a brief preview of what the model returned (truncated, no raw grid dump)
      final preview = content.length > 200 ? '${content.substring(0, 200)}...' : content;
      chat.addAssistantMessage(
        'Couldn\'t extract a valid grid from the response.\n\n'
        '> ${preview.replaceAll('\n', '\n> ')}\n\n'
        '*${resp.totalTokens} tokens used*$hint',
      );
      return;
    }

    // Step 5: Create tile via backend → validate + render
    // Use actual grid dimensions — the model may have returned a different size.
    final finalLines = grid.split('\n').where((l) => l.trim().isNotEmpty).toList();
    final finalW = finalLines.isNotEmpty ? finalLines.first.length : 0;
    final finalH = finalLines.length;
    final finalSize = '${finalW}x$finalH';

    final existingNames = ref.read(backendProvider).tiles.map((t) => t.name).toSet();
    final tileName = uniqueTileName(generateTileName(prompt), existingNames);
    final palette = ctx['palette'] as String? ?? ensuredPalette;
    final createResp = await ref.read(backendProvider.notifier).createTile(
      name: tileName,
      palette: palette,
      size: finalSize,
      grid: grid,
    );

    if (createResp.containsKey('error')) {
      chat.addAssistantMessage(
        'Tile creation failed: ${createResp['error']}',
      );
      return;
    }

    // Step 6: Get preview (fallback to renderTile if createResp lacks one)
    final previewB64 = await _getPreviewB64(
      tileName, createResp['preview'] as String?,
    );

    setState(() {
      _pendingTileName = tileName;
    });

    final validationInfo = createResp['validation'] as Map<String, dynamic>?;
    final isValid = validationInfo?['valid'] as bool? ?? true;

    final previews = <({String name, String base64})>[];
    if (previewB64 != null) {
      previews.add((name: tileName, base64: previewB64));
    }

    // Rate the tile aesthetically
    final rating = await ref.read(backendProvider.notifier).rateTile(tileName);
    final stars = rating != null
        ? '${'★' * (rating['overall'] as int? ?? 0)}${'☆' * (5 - (rating['overall'] as int? ?? 0))} ${rating['assessment'] ?? ''}'
        : '';

    chat.addAssistantMessage(
      '**Generated: `$tileName`** ($finalSize)\n\n'
      '${isValid ? 'Validation passed.' : 'Validation warnings — check the validation panel.'}'
      '${stars.isNotEmpty ? '\n\nRating: $stars' : ''}\n\n'
      '*${resp.totalTokens} tokens*',
      previewImages: previews,
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

    final previewB64 = await _getPreviewB64(
      tileName, resp['preview_b64'] as String?,
    );

    setState(() {
      _pendingTileName = tileName;
    });

    final generated = resp['generated'] == true;
    final previews = <({String name, String base64})>[];
    if (previewB64 != null) {
      previews.add((name: tileName, base64: previewB64));
    }

    // Rate the tile
    final loraRating = await ref.read(backendProvider.notifier).rateTile(tileName);
    final loraStars = loraRating != null
        ? '${'★' * (loraRating['overall'] as int? ?? 0)}${'☆' * (5 - (loraRating['overall'] as int? ?? 0))} ${loraRating['assessment'] ?? ''}'
        : '';

    chat.addAssistantMessage(
      '**Generated: `$tileName`** ($sizeStr)\n\n'
      '${generated ? 'Generated on-device with LoRA adapter.' : 'Created.'}'
      '${loraStars.isNotEmpty ? '\n\nRating: $loraStars' : ''}',
      previewImages: previews,
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

  // ── AI Commands ─────────────────────────────────────────

  bool _isAiCommand(String text) {
    final lower = text.toLowerCase().trim();
    return lower.startsWith('make it tile') ||
        lower.startsWith('fix edges') ||
        lower.startsWith('make tileable') ||
        lower.startsWith('auto-tag') ||
        lower.startsWith('auto tag') ||
        lower.startsWith('tag all') ||
        lower.contains('style transfer') ||
        lower.startsWith('restyle') ||
        lower.startsWith('make this look like') ||
        lower.startsWith('shift to') ||
        lower.startsWith('inpaint') ||
        lower.startsWith('fill this with');
  }

  Future<void> _handleAiCommand(String text) async {
    final lower = text.toLowerCase().trim();
    final chat = ref.read(chatProvider.notifier);
    final backend = ref.read(backendProvider);

    if (!backend.isConnected) {
      chat.addAssistantMessage('Engine not connected.');
      return;
    }

    // ── Make it Tile ──
    if (lower.startsWith('make it tile') ||
        lower.startsWith('fix edges') ||
        lower.startsWith('make tileable')) {
      await _handleMakeItTile();
      return;
    }

    // ── Auto-Tag ──
    if (lower.startsWith('auto-tag') ||
        lower.startsWith('auto tag') ||
        lower.startsWith('tag all')) {
      await _handleAutoTag();
      return;
    }

    // ── Style Transfer ──
    if (lower.contains('style transfer') ||
        lower.startsWith('restyle') ||
        lower.startsWith('make this look like') ||
        lower.startsWith('shift to')) {
      await _handleStyleTransfer(text);
      return;
    }

    // ── Inpaint ──
    if (lower.startsWith('inpaint') || lower.startsWith('fill this with')) {
      await _handleInpaint(text);
      return;
    }
  }

  Future<void> _handleMakeItTile() async {
    final chat = ref.read(chatProvider.notifier);
    final tiles = ref.read(backendProvider).tiles;

    if (_pendingTileName == null && tiles.isEmpty) {
      chat.addAssistantMessage('No tile selected. Generate or create a tile first.');
      return;
    }

    final tileName = _pendingTileName ?? tiles.last.name;
    chat.addAssistantMessage('Making **`$tileName`** tileable...', isStatus: true);
    _scrollToBottom();

    // Get the tile's grid from backend
    final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
      prompt: 'Fix the edges of tile "$tileName" to make it seamlessly tileable. '
          'The tile must repeat horizontally and vertically with no visible seams. '
          'Keep the interior design intact, only adjust border pixels for continuity.',
    );

    final resp = await ref.read(claudeProvider.notifier).generateTile(
      systemPrompt: ctx['system_prompt'] as String? ?? '',
      userPrompt: ctx['user_prompt'] as String? ?? '',
    );

    if (resp.isError) {
      chat.addAssistantMessage('Failed: ${resp.errorMessage}');
      return;
    }

    final grid = extractGrid(resp.content);
    if (grid == null) {
      chat.addAssistantMessage('Could not extract a valid grid from the response.');
      return;
    }

    final newName = '${tileName}_tileable';
    final canvasSize = ref.read(canvasProvider).canvasSize;
    final sizeStr = '${canvasSize.width}x${canvasSize.height}';
    final pal = ctx['palette'] as String? ?? await _ensurePalette();
    if (pal == null) return;
    final createResp = await ref.read(backendProvider.notifier).createTile(
      name: newName,
      palette: pal,
      size: sizeStr,
      grid: grid,
    );

    if (createResp.containsKey('error')) {
      chat.addAssistantMessage('Tile creation failed: ${createResp['error']}');
      return;
    }

    final previewB64 = await _getPreviewB64(newName, createResp['preview'] as String?);
    setState(() {
      _pendingTileName = newName;
    });
    final previews = <({String name, String base64})>[];
    if (previewB64 != null) previews.add((name: newName, base64: previewB64));
    chat.addAssistantMessage(
      '**Created tileable version: `$newName`**',
      previewImages: previews,
    );
    _scrollToBottom();
  }

  Future<void> _handleAutoTag() async {
    final chat = ref.read(chatProvider.notifier);
    final tiles = ref.read(backendProvider).tiles;

    if (tiles.isEmpty) {
      chat.addAssistantMessage('No tiles in session to tag.');
      return;
    }

    chat.addAssistantMessage('Auto-tagging ${tiles.length} tiles...', isStatus: true);
    _scrollToBottom();

    // Use the LLM to analyze all tiles at once
    final tileNames = tiles.map((t) => t.name).join(', ');
    final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
      prompt: 'Analyze these tiles and suggest semantic tags for each: $tileNames. '
          'For each tile, provide: tags (e.g. wall, floor, corner, decoration), '
          'a target_layer (background/terrain/walls/platform/foreground/effects), '
          'and a brief description. Format as a list.',
    );

    final resp = await ref.read(claudeProvider.notifier).generateTile(
      systemPrompt: ctx['system_prompt'] as String? ?? '',
      userPrompt: ctx['user_prompt'] as String? ?? '',
    );

    if (resp.isError) {
      chat.addAssistantMessage('Auto-tag failed: ${resp.errorMessage}');
      return;
    }

    chat.addAssistantMessage(
      '**Auto-tag results:**\n\n${resp.content}\n\n'
      '*Note: tag updates need to be applied manually via the engine for now.*',
    );
    _scrollToBottom();
  }

  Future<void> _handleStyleTransfer(String text) async {
    final chat = ref.read(chatProvider.notifier);
    final tiles = ref.read(backendProvider).tiles;

    if (_pendingTileName == null && tiles.isEmpty) {
      chat.addAssistantMessage('No tile to restyle. Generate or create a tile first.');
      return;
    }

    final tileName = _pendingTileName ?? tiles.last.name;
    chat.addAssistantMessage('Restyling **`$tileName`**...', isStatus: true);
    _scrollToBottom();

    final canvasSize = ref.read(canvasProvider).canvasSize;
    final sizeStr = '${canvasSize.width}x${canvasSize.height}';

    final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
      prompt: 'Restyle tile "$tileName" to match this style: $text. '
          'Use only the palette symbols provided. Output only the grid.',
      size: sizeStr,
    );

    final resp = await ref.read(claudeProvider.notifier).generateTile(
      systemPrompt: ctx['system_prompt'] as String? ?? '',
      userPrompt: ctx['user_prompt'] as String? ?? '',
    );

    if (resp.isError) {
      chat.addAssistantMessage('Style transfer failed: ${resp.errorMessage}');
      return;
    }

    final grid = extractGrid(resp.content);
    if (grid == null) {
      chat.addAssistantMessage('Could not extract a valid grid from the response.');
      return;
    }

    final newName = '${tileName}_restyled';
    final pal2 = ctx['palette'] as String? ?? await _ensurePalette();
    if (pal2 == null) return;
    final createResp = await ref.read(backendProvider.notifier).createTile(
      name: newName,
      palette: pal2,
      size: sizeStr,
      grid: grid,
    );

    if (createResp.containsKey('error')) {
      chat.addAssistantMessage('Tile creation failed: ${createResp['error']}');
      return;
    }

    final restylePreview = await _getPreviewB64(newName, createResp['preview'] as String?);
    setState(() {
      _pendingTileName = newName;
    });
    final restylePreviews = <({String name, String base64})>[];
    if (restylePreview != null) restylePreviews.add((name: newName, base64: restylePreview));
    chat.addAssistantMessage(
      '**Restyled: `$newName`**',
      previewImages: restylePreviews,
    );
    _scrollToBottom();
  }

  Future<void> _handleInpaint(String text) async {
    final chat = ref.read(chatProvider.notifier);
    final sel = ref.read(selectionProvider);

    if (!sel.hasSelection) {
      chat.addAssistantMessage(
        'Select a region first with the **Select tool (S)**, then describe what to fill it with.',
      );
      return;
    }

    final tiles = ref.read(backendProvider).tiles;
    if (_pendingTileName == null && tiles.isEmpty) {
      chat.addAssistantMessage('No tile context for inpainting.');
      return;
    }

    final tileName = _pendingTileName ?? tiles.last.name;
    chat.addAssistantMessage(
      'Inpainting region (${sel.x},${sel.y})→(${sel.x + sel.width},${sel.y + sel.height}) '
      'in **`$tileName`**...',
      isStatus: true,
    );
    _scrollToBottom();

    final canvasSize = ref.read(canvasProvider).canvasSize;
    final sizeStr = '${canvasSize.width}x${canvasSize.height}';

    final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
      prompt: 'Modify tile "$tileName": in the region from row ${sel.y} to ${sel.y + sel.height - 1}, '
          'columns ${sel.x} to ${sel.x + sel.width - 1}, replace those pixels with: $text. '
          'Keep all pixels outside this region exactly the same. Output the complete grid.',
      size: sizeStr,
    );

    final resp = await ref.read(claudeProvider.notifier).generateTile(
      systemPrompt: ctx['system_prompt'] as String? ?? '',
      userPrompt: ctx['user_prompt'] as String? ?? '',
    );

    if (resp.isError) {
      chat.addAssistantMessage('Inpaint failed: ${resp.errorMessage}');
      return;
    }

    final grid = extractGrid(resp.content);
    if (grid == null) {
      chat.addAssistantMessage('Could not extract a valid grid from the response.');
      return;
    }

    final newName = '${tileName}_inpainted';
    final pal3 = ctx['palette'] as String? ?? await _ensurePalette();
    if (pal3 == null) return;
    final createResp = await ref.read(backendProvider.notifier).createTile(
      name: newName,
      palette: pal3,
      size: sizeStr,
      grid: grid,
    );

    if (createResp.containsKey('error')) {
      chat.addAssistantMessage('Tile creation failed: ${createResp['error']}');
      return;
    }

    final inpaintPreview = await _getPreviewB64(newName, createResp['preview'] as String?);
    setState(() {
      _pendingTileName = newName;
    });
    final inpaintPreviews = <({String name, String base64})>[];
    if (inpaintPreview != null) inpaintPreviews.add((name: newName, base64: inpaintPreview));
    chat.addAssistantMessage(
      '**Inpainted: `$newName`**',
      previewImages: inpaintPreviews,
    );
    _scrollToBottom();
  }

  /// Build a tile preview card for inline display in chat messages.
  Widget _buildTilePreviewCard(
    ThemeData theme,
    ({String name, String base64}) img,
    int totalCount,
  ) {
    final isPending = _pendingTileName == img.name;
    final isReferenced = _referencedTile?.name == img.name;
    final isSingle = totalCount == 1;

    // Single tile: full card with name, actions, accept/reject.
    // Multi tile: compact thumbnail — click to preview, tooltip for name.
    if (!isSingle) {
      return _buildCompactTileThumb(theme, img, isReferenced);
    }

    return Container(
      constraints: const BoxConstraints(maxWidth: 200),
      decoration: BoxDecoration(
        color: StudioTheme.recessedBg,
        borderRadius: BorderRadius.circular(6),
        border: Border.all(
          color: isPending
              ? theme.colorScheme.primary.withValues(alpha: 0.5)
              : isReferenced
                  ? theme.colorScheme.primary
                  : theme.dividerColor.withValues(alpha: 0.5),
          width: isReferenced ? 2 : 1,
        ),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          // Tile image — click to preview on canvas
          _HoverablePreview(
            onTap: () => _previewTileOnCanvas(img.name),
            child: Container(
              padding: const EdgeInsets.all(8),
              decoration: const BoxDecoration(
                color: Color(0xFF0E0E0E),
                borderRadius: BorderRadius.vertical(top: Radius.circular(5)),
              ),
              child: Center(
                child: Image.memory(
                  base64Decode(img.base64),
                  width: 96, height: 96,
                  filterQuality: FilterQuality.none,
                  fit: BoxFit.contain,
                ),
              ),
            ),
          ),

          // Footer: name + actions
          Padding(
            padding: const EdgeInsets.fromLTRB(8, 5, 4, 5),
            child: Row(
              children: [
                Expanded(
                  child: Text(
                    img.name,
                    style: theme.textTheme.bodySmall!.copyWith(
                      fontSize: 9,
                      color: theme.colorScheme.onSurface.withValues(alpha: 0.7),
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                if (isPending) ...[
                  _TinyIconBtn(
                    icon: Icons.check_circle_outline,
                    tooltip: 'Add to session',
                    color: StudioTheme.success,
                    onTap: _acceptTile,
                  ),
                  _TinyIconBtn(
                    icon: Icons.cancel_outlined,
                    tooltip: 'Reject',
                    color: StudioTheme.error,
                    onTap: _rejectTile,
                  ),
                  const SizedBox(width: 2),
                ],
                _TinyIconBtn(
                  icon: Icons.visibility_outlined,
                  tooltip: 'Preview on canvas',
                  onTap: () => _previewTileOnCanvas(img.name),
                ),
                _TinyIconBtn(
                  icon: Icons.link,
                  tooltip: 'Use as reference',
                  isActive: isReferenced,
                  onTap: () {
                    final tiles = ref.read(backendProvider).tiles;
                    final match = tiles.where((t) => t.name == img.name);
                    setState(() {
                      _referencedTile = isReferenced
                          ? null
                          : (match.isNotEmpty ? match.first : TileInfo(name: img.name));
                    });
                  },
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }

  /// Compact thumbnail for multi-tile previews (tileset loads).
  Widget _buildCompactTileThumb(
    ThemeData theme,
    ({String name, String base64}) img,
    bool isReferenced,
  ) {
    return Tooltip(
      message: img.name,
      child: _HoverablePreview(
        onTap: () => _previewTileOnCanvas(img.name),
        onSecondaryTap: () {
          final tiles = ref.read(backendProvider).tiles;
          final match = tiles.where((t) => t.name == img.name);
          setState(() {
            _referencedTile = isReferenced
                ? null
                : (match.isNotEmpty ? match.first : TileInfo(name: img.name));
          });
        },
        child: Container(
          decoration: BoxDecoration(
            color: const Color(0xFF121212),
            borderRadius: BorderRadius.circular(4),
            border: Border.all(
              color: isReferenced
                  ? theme.colorScheme.primary
                  : theme.dividerColor.withValues(alpha: 0.4),
              width: isReferenced ? 2 : 1,
            ),
          ),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(3),
            child: Image.memory(
              base64Decode(img.base64),
              width: 56, height: 56,
              filterQuality: FilterQuality.none,
              fit: BoxFit.contain,
            ),
          ),
        ),
      ),
    );
  }

  /// Get a preview base64 for a tile — tries createResp, renderTile, then tile list.
  Future<String?> _getPreviewB64(String tileName, String? fromCreateResp) async {
    if (fromCreateResp != null) return fromCreateResp;
    // Fallback 1: render via backend API
    final rendered = await ref.read(backendProvider.notifier).renderTile(tileName, scale: 8);
    if (rendered != null) return rendered;
    // Fallback 2: check if tile list already has a preview (from refreshTiles)
    final tiles = ref.read(backendProvider).tiles;
    final match = tiles.where((t) => t.name == tileName);
    return match.isNotEmpty ? match.first.previewBase64 : null;
  }

  /// Load a tile onto the canvas for preview.
  Future<void> _previewTileOnCanvas(String tileName) async {
    final result = await ref.read(backendProvider.notifier).getTilePixels(tileName);
    if (result == null || !mounted) return;
    ref.read(canvasProvider.notifier).loadTilePixels(
      result.pixels,
      result.width,
      result.height,
    );
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

      // Check if engine suggests retraining
      _checkRetrainSuggestion();
    }
    setState(() {
      _pendingTileName = null;

    });
    _scrollToBottom();
  }

  /// Reject the pending tile — show reason selector, record feedback.
  void _rejectTile() {
    if (_pendingTileName == null) return;
    _showRejectReasonPicker();
  }

  void _showRejectReasonPicker() {
    const reasons = [
      ('too_sparse', 'Too sparse', Icons.blur_on),
      ('too_dense', 'Too dense', Icons.grid_on),
      ('wrong_style', 'Wrong style', Icons.style),
      ('bad_edges', 'Bad edges', Icons.crop),
      ('palette_violation', 'Palette issue', Icons.palette_outlined),
      ('bad_composition', 'Bad composition', Icons.dashboard_outlined),
      ('looks_bad', 'Looks bad', Icons.thumb_down_outlined),
    ];

    final theme = Theme.of(context);
    final customController = TextEditingController();

    showModalBottomSheet(
      context: context,
      backgroundColor: Colors.transparent,
      isScrollControlled: true,
      builder: (ctx) => Padding(
        padding: EdgeInsets.only(
          bottom: MediaQuery.of(ctx).viewInsets.bottom,
        ),
        child: Container(
          margin: const EdgeInsets.all(12),
          decoration: BoxDecoration(
            color: StudioTheme.recessedBg,
            borderRadius: BorderRadius.circular(12),
            border: Border.all(color: theme.dividerColor.withValues(alpha: 0.3)),
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(14, 12, 14, 8),
                child: Text(
                  'Why reject?',
                  style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 10,
                    color: theme.colorScheme.onSurface.withValues(alpha: 0.5),
                    fontWeight: FontWeight.w600,
                    letterSpacing: 0.5,
                  ),
                ),
              ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: Wrap(
                  spacing: 6,
                  runSpacing: 6,
                  children: [
                    for (final (key, label, icon) in reasons)
                      _RejectChip(
                        icon: icon,
                        label: label,
                        onTap: () {
                          Navigator.of(ctx).pop();
                          _doReject(key);
                        },
                      ),
                  ],
                ),
              ),
              const SizedBox(height: 8),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: TextField(
                  controller: customController,
                  style: theme.textTheme.bodySmall!.copyWith(fontSize: 11),
                  decoration: InputDecoration(
                    hintText: 'Or type your own reason...',
                    hintStyle: theme.textTheme.bodySmall!.copyWith(
                      fontSize: 10,
                      color: theme.colorScheme.onSurface.withValues(alpha: 0.3),
                    ),
                    isDense: true,
                    contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                    filled: true,
                    fillColor: theme.dividerColor.withValues(alpha: 0.1),
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(8),
                      borderSide: BorderSide.none,
                    ),
                    suffixIcon: IconButton(
                      icon: Icon(Icons.send, size: 14,
                        color: theme.colorScheme.onSurface.withValues(alpha: 0.4)),
                      onPressed: () {
                        final text = customController.text.trim();
                        Navigator.of(ctx).pop();
                        _doReject(text.isEmpty ? null : text);
                      },
                    ),
                    suffixIconConstraints: const BoxConstraints(maxHeight: 30, maxWidth: 30),
                  ),
                  onSubmitted: (text) {
                    Navigator.of(ctx).pop();
                    _doReject(text.trim().isEmpty ? null : text.trim());
                  },
                ),
              ),
              const SizedBox(height: 10),
            ],
          ),
        ),
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

      // Check if engine suggests retraining
      _checkRetrainSuggestion();
    }
    setState(() {
      _pendingTileName = null;

    });
    _scrollToBottom();
  }

  /// Check if the engine suggests retraining based on feedback stats.
  Future<void> _checkRetrainSuggestion() async {
    final stats = await ref.read(backendProvider.notifier).backend.feedbackStats();
    final suggest = stats['suggest_retrain'] as bool? ?? false;
    if (suggest && mounted) {
      final hint = stats['retrain_hint'] as String? ?? 'Consider retraining your adapter.';
      ref.read(chatProvider.notifier).addAssistantMessage(
        '**Retrain suggested:** $hint\n\n'
        'Open the Style Scanner to retrain with your feedback.',
      );
    }
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
    final systemPrompt = KnowledgeBase.buildSystemPrompt(
      backendContext: backendContext,
      styleFragment: style.toPromptFragment(),
    );

    // Fire 3 generation calls in parallel
    final existingNames = ref.read(backendProvider).tiles.map((t) => t.name).toSet();
    final futures = List.generate(3, (i) async {
      final resp = await ref.read(claudeProvider.notifier).service.generate(
        systemPrompt: systemPrompt,
        userPrompt: userPrompt,
        temperature: 0.5 + (i * 0.15), // slight temperature variation
      );
      if (resp.isError) return null;

      final grid = extractGrid(resp.content);
      if (grid == null) return null;

      // Use actual grid dimensions — the model may return a different size.
      final gridLines = grid.split('\n').where((l) => l.trim().isNotEmpty).toList();
      final gridW = gridLines.isNotEmpty ? gridLines.first.length : 0;
      final gridH = gridLines.length;
      final gridSize = '${gridW}x$gridH';

      final tileName = uniqueTileName('${generateTileName(prompt)}_v${i + 1}', existingNames);
      final varPal = ctx['palette'] as String? ?? await _ensurePalette();
      if (varPal == null) return null;
      final createResp = await ref.read(backendProvider.notifier).createTile(
        name: tileName,
        palette: varPal,
        size: gridSize,
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

  Future<void> _handleChat(String text, {TileInfo? refTile}) async {
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
      // Build system prompt with knowledge base + session context.
      // Pass tile type (not chat) so the palette symbol table is included —
      // if the LLM decides to return a grid, it needs valid symbols.
      final canvasSize = ref.read(canvasProvider).canvasSize;
      final chatSize = '${canvasSize.width}x${canvasSize.height}';
      final ctx = await ref.read(backendProvider.notifier).getGenerationContext(
        prompt: text,
        size: chatSize,
      );
      final backendCtx = ctx['system_prompt'] as String? ?? '';
      final style = ref.read(styleProvider);
      final systemPrompt = KnowledgeBase.buildSystemPrompt(
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

      // Inject referenced tile context into the last user message.
      if (refTile != null && recentMessages.isNotEmpty) {
        final last = recentMessages.last;
        if (last.role == 'user') {
          final edges = refTile.edgeClasses?.entries.map((e) => '${e.key}=${e.value}').join(', ') ?? 'unknown';
          final tags = refTile.tags.isNotEmpty ? refTile.tags.join(', ') : 'none';
          recentMessages[recentMessages.length - 1] = LlmMessage(
            role: 'user',
            content: '[Reference tile: ${refTile.name}, size: ${refTile.size ?? 'unknown'}, '
                'edges: $edges, tags: $tags]\n\n${last.content}',
          );
        }
      }

      final resp = await ref.read(claudeProvider.notifier).chat(
        systemPrompt: systemPrompt,
        messages: recentMessages,
      );

      if (resp.isError) {
        chat.addAssistantMessage('Error: ${resp.errorMessage}');
      } else {
        // ── PAX interception: detect tile content in chat responses ──
        final content = resp.content;

        if (_isPaxSource(content)) {
          // Full PAX tileset — load into session (merges, no overwrite)
          final paxSource = _extractPaxSource(content);
          chat.addAssistantMessage('Loading tileset...', isStatus: true);
          _scrollToBottom();

          final tilesBefore = ref.read(backendProvider).tiles.map((t) => t.name).toSet();
          final loadResp = await ref.read(backendProvider.notifier).loadSource(paxSource);

          if (loadResp.containsKey('error')) {
            chat.addAssistantMessage(
              'Failed to load PAX: ${loadResp['error']}\n\n'
              '*${resp.totalTokens} tokens used*',
            );
          } else {
            final allTiles = ref.read(backendProvider).tiles;
            final newTiles = allTiles.where((t) => !tilesBefore.contains(t.name)).toList();
            final previews = <({String name, String base64})>[];
            for (final t in newTiles) {
              if (t.previewBase64 != null) {
                previews.add((name: t.name, base64: t.previewBase64!));
              }
            }
            chat.addAssistantMessage(
              '**Loaded ${newTiles.length} tile${newTiles.length == 1 ? '' : 's'}** '
              'into session (${allTiles.length} total)\n\n'
              '*${resp.totalTokens} tokens*',
              previewImages: previews,
            );
          }
        } else {
          // Check for a single grid embedded in the response
          final grid = extractGrid(content);

          if (grid != null) {
            // Single tile detected — use actual grid dimensions.
            final gridLines = grid.split('\n').where((l) => l.trim().isNotEmpty).toList();
            final gridW = gridLines.isNotEmpty ? gridLines.first.length : 0;
            final gridH = gridLines.length;
            final gridSize = '${gridW}x$gridH';

            final genCtx = await ref.read(backendProvider.notifier).getGenerationContext(
              prompt: text,
              size: gridSize,
            );
            final existingNames = ref.read(backendProvider).tiles.map((t) => t.name).toSet();
            final tileName = uniqueTileName(generateTileName(text), existingNames);

            final chatPalette = genCtx['palette'] as String? ?? await _ensurePalette();
            if (chatPalette == null) return;
            final createResp = await ref.read(backendProvider.notifier).createTile(
              name: tileName,
              palette: chatPalette,
              size: gridSize,
              grid: grid,
            );

            if (createResp.containsKey('error')) {
              // Creation failed — show error + the raw grid so the user
              // can see what the LLM produced and diagnose the issue.
              final truncGrid = grid.length > 400 ? '${grid.substring(0, 400)}...' : grid;
              chat.addAssistantMessage(
                'Tile creation failed: ${createResp['error']}\n\n'
                '```\n$truncGrid\n```\n\n'
                '*${resp.totalTokens} tokens*',
              );
            } else {
              final previewB64 = await _getPreviewB64(
                tileName, createResp['preview'] as String?,
              );

              setState(() {
                _pendingTileName = tileName;
                _lastGenerationPrompt = text;
              });

              final previews = <({String name, String base64})>[];
              if (previewB64 != null) {
                previews.add((name: tileName, base64: previewB64));
              }

              chat.addAssistantMessage(
                '**Generated: `$tileName`** ($gridSize)\n\n'
                '*${resp.totalTokens} tokens*',
                previewImages: previews,
              );
            }
          } else {
            // No PAX content — plain text chat response
            chat.addAssistantMessage(content);
          }
        }
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
  @override
  void dispose() {
    _controller.dispose();
    _scrollController.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  void _historyUp() {
    if (_inputHistory.isEmpty) return;
    if (_historyIndex == -1) {
      _savedInput = _controller.text;
      _historyIndex = _inputHistory.length - 1;
    } else if (_historyIndex > 0) {
      _historyIndex--;
    }
    _controller.text = _inputHistory[_historyIndex];
    _controller.selection = TextSelection.collapsed(offset: _controller.text.length);
  }

  void _historyDown() {
    if (_historyIndex == -1) return;
    if (_historyIndex < _inputHistory.length - 1) {
      _historyIndex++;
      _controller.text = _inputHistory[_historyIndex];
    } else {
      _historyIndex = -1;
      _controller.text = _savedInput;
    }
    _controller.selection = TextSelection.collapsed(offset: _controller.text.length);
  }

  @override
  Widget build(BuildContext context) {
    final messages = ref.watch(chatProvider);
    final theme = Theme.of(context);
    final backend = ref.watch(backendProvider);
    final claude = ref.watch(claudeProvider);
    final isGenerating = claude.isGenerating;

    // Scroll to bottom when typing indicator appears
    if (isGenerating && !_wasGenerating) {
      _scrollToBottom();
    }
    _wasGenerating = isGenerating;

    return Container(
      width: 260,
      decoration: StudioTheme.panelDecoration,
      child: Column(
        children: [
          // ── Header ──
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
            decoration: const BoxDecoration(
              border: Border(bottom: StudioTheme.panelBorder),
            ),
            child: Row(
              children: [
                Container(
                  width: 18, height: 18,
                  decoration: BoxDecoration(
                    gradient: LinearGradient(
                      colors: [
                        theme.colorScheme.primary.withValues(alpha: 0.3),
                        theme.colorScheme.primary.withValues(alpha: 0.1),
                      ],
                    ),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: Icon(Icons.auto_awesome, size: 11, color: theme.colorScheme.primary),
                ),
                const SizedBox(width: 6),
                Text('AI EXPERT', style: theme.textTheme.titleSmall!.copyWith(fontSize: 10)),
                const Spacer(),
                _StatusDot(
                  color: backend.isConnected ? StudioTheme.success : StudioTheme.separatorColor,
                  tooltip: backend.isConnected ? 'Engine connected' : 'Engine offline',
                ),
                const SizedBox(width: 4),
                _StatusDot(
                  color: claude.hasApiKey ? StudioTheme.success : StudioTheme.warning,
                  tooltip: claude.hasApiKey ? 'API key set' : 'No API key',
                ),
                const SizedBox(width: 6),
                _IconBtn(
                  icon: Icons.delete_outline,
                  onTap: () {
                    ref.read(chatProvider.notifier).clear();
                    setState(() {
                      _pendingTileName = null;
                      _lastGenerationPrompt = null;
                    });
                  },
                  tooltip: 'Clear chat',
                ),
              ],
            ),
          ),

          // ── Messages ──
          Expanded(
            child: messages.isEmpty
                ? _buildWelcomeState(theme, claude, backend)
                : ListView.builder(
                    controller: _scrollController,
                    padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 8),
                    itemCount: messages.length + (isGenerating ? 1 : 0),
                    itemBuilder: (context, index) {
                      // Typing indicator as last item when generating
                      if (index == messages.length) {
                        return _buildTypingIndicator(theme);
                      }
                      final msg = messages[index];
                      if (msg.isStatus) {
                        return _buildStatusChip(theme, msg.content);
                      }
                      return msg.role == 'user'
                          ? _buildUserBubble(theme, msg)
                          : _buildAssistantBubble(theme, msg);
                    },
                  ),
          ),

          // ── Variation strip ──
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
                      Text('VARIATIONS', style: theme.textTheme.titleSmall!.copyWith(fontSize: 9)),
                      if (_isGeneratingVariations) ...[
                        const SizedBox(width: 8),
                        SizedBox(
                          width: 10, height: 10,
                          child: CircularProgressIndicator(
                            strokeWidth: 1.5,
                            color: theme.colorScheme.primary,
                          ),
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
                      for (var i = _variations.length; i < 3 && _isGeneratingVariations; i++) ...[
                        if (i > 0 || _variations.isNotEmpty) const SizedBox(width: 6),
                        Container(
                          width: 64, height: 64,
                          decoration: BoxDecoration(
                            borderRadius: BorderRadius.circular(4),
                            border: Border.all(color: theme.dividerColor),
                            color: StudioTheme.codeBg,
                          ),
                          child: Center(
                            child: SizedBox(
                              width: 14, height: 14,
                              child: CircularProgressIndicator(
                                strokeWidth: 1.5,
                                color: theme.colorScheme.primary,
                              ),
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

          // ── Pending tile bar (compact) ──
          if (_pendingTileName != null && _variations.isEmpty)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
              decoration: BoxDecoration(
                border: const Border(top: StudioTheme.panelBorder),
                color: theme.colorScheme.primary.withValues(alpha: 0.06),
              ),
              child: Row(
                children: [
                  Icon(Icons.auto_awesome, size: 11, color: theme.colorScheme.primary),
                  const SizedBox(width: 5),
                  Expanded(
                    child: Text(
                      _pendingTileName!,
                      style: theme.textTheme.bodySmall!.copyWith(
                        fontSize: 9,
                        color: theme.colorScheme.primary,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  _SmallActionBtn(
                    icon: Icons.shuffle,
                    label: 'Vary',
                    onTap: isGenerating ? null : _generateVariation,
                  ),
                  const SizedBox(width: 4),
                  _SmallActionBtn(
                    icon: Icons.grid_view,
                    label: '3x',
                    onTap: isGenerating ? null : _generateVariations,
                  ),
                ],
              ),
            ),

          // ── Reference chip ──
          if (_referencedTile != null)
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
              decoration: BoxDecoration(
                border: const Border(top: StudioTheme.panelBorder),
                color: theme.colorScheme.primary.withValues(alpha: 0.06),
              ),
              child: Row(
                children: [
                  if (_referencedTile!.previewBase64 != null)
                    Padding(
                      padding: const EdgeInsets.only(right: 5),
                      child: ClipRRect(
                        borderRadius: BorderRadius.circular(2),
                        child: Image.memory(
                          base64Decode(_referencedTile!.previewBase64!),
                          width: 18, height: 18,
                          filterQuality: FilterQuality.none,
                          fit: BoxFit.contain,
                        ),
                      ),
                    ),
                  Icon(Icons.link, size: 10, color: theme.colorScheme.primary),
                  const SizedBox(width: 4),
                  Expanded(
                    child: Text(
                      _referencedTile!.name,
                      style: theme.textTheme.bodySmall!.copyWith(
                        fontSize: 9,
                        color: theme.colorScheme.primary,
                      ),
                      overflow: TextOverflow.ellipsis,
                    ),
                  ),
                  InkWell(
                    onTap: () => setState(() => _referencedTile = null),
                    borderRadius: BorderRadius.circular(8),
                    child: Icon(Icons.close, size: 12, color: theme.textTheme.bodySmall?.color),
                  ),
                ],
              ),
            ),

          // ── Composer ──
          Container(
            padding: const EdgeInsets.all(8),
            decoration: const BoxDecoration(
              border: Border(top: StudioTheme.panelBorder),
            ),
            child: Container(
              decoration: BoxDecoration(
                color: StudioTheme.recessedBg,
                borderRadius: BorderRadius.circular(12),
                border: Border.all(
                  color: _focusNode.hasFocus
                      ? theme.colorScheme.primary.withValues(alpha: 0.5)
                      : theme.dividerColor.withValues(alpha: 0.4),
                ),
              ),
              child: Column(
                children: [
                  TextField(
                    controller: _controller,
                    focusNode: _focusNode,
                    enabled: !isGenerating,
                    style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                    maxLines: 4,
                    minLines: 1,
                    textInputAction: TextInputAction.newline,
                    decoration: InputDecoration(
                      hintText: isGenerating
                          ? 'Generating...'
                          : 'Message...',
                      hintStyle: theme.textTheme.bodySmall!.copyWith(fontSize: 11),
                      isDense: true,
                      contentPadding: const EdgeInsets.fromLTRB(12, 10, 12, 4),
                      border: InputBorder.none,
                    ),
                  ),
                  // Bottom controls row
                  Padding(
                    padding: const EdgeInsets.fromLTRB(4, 0, 4, 4),
                    child: Row(
                      children: [
                        // Model indicator
                        Padding(
                          padding: const EdgeInsets.only(left: 4),
                          child: Text(
                            claude.model.split('-').take(2).join(' '),
                            style: theme.textTheme.bodySmall!.copyWith(
                              fontSize: 8,
                              color: theme.disabledColor,
                            ),
                          ),
                        ),
                        const Spacer(),
                        // Send button
                        AnimatedContainer(
                          duration: const Duration(milliseconds: 150),
                          width: 26, height: 26,
                          decoration: BoxDecoration(
                            color: isGenerating
                                ? Colors.transparent
                                : theme.colorScheme.primary.withValues(alpha: 0.15),
                            borderRadius: BorderRadius.circular(8),
                          ),
                          child: isGenerating
                              ? Center(
                                  child: SizedBox(
                                    width: 14, height: 14,
                                    child: CircularProgressIndicator(
                                      strokeWidth: 1.5,
                                      color: theme.colorScheme.primary,
                                    ),
                                  ),
                                )
                              : InkWell(
                                  onTap: _send,
                                  borderRadius: BorderRadius.circular(8),
                                  child: Icon(
                                    Icons.arrow_upward,
                                    size: 14,
                                    color: theme.colorScheme.primary,
                                  ),
                                ),
                        ),
                      ],
                    ),
                  ),
                ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  // ── Message Bubble Builders ────────────────────────────────

  /// Welcome state shown when chat is empty.
  Widget _buildWelcomeState(ThemeData theme, LlmState claude, BackendState backend) {
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Container(
              width: 40, height: 40,
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: [
                    theme.colorScheme.primary.withValues(alpha: 0.2),
                    theme.colorScheme.primary.withValues(alpha: 0.05),
                  ],
                ),
                borderRadius: BorderRadius.circular(12),
              ),
              child: Icon(
                Icons.auto_awesome,
                size: 20,
                color: theme.colorScheme.primary,
              ),
            ),
            const SizedBox(height: 12),
            Text(
              'AI Tile Expert',
              style: theme.textTheme.bodyMedium!.copyWith(
                fontSize: 12,
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 6),
            Text(
              'Generate tiles, get feedback,\nor ask about pixel art.',
              textAlign: TextAlign.center,
              style: theme.textTheme.bodySmall!.copyWith(
                fontSize: 10,
                height: 1.4,
              ),
            ),
            if (!backend.isConnected || !claude.hasApiKey) ...[
              const SizedBox(height: 12),
              Container(
                padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                decoration: BoxDecoration(
                  color: StudioTheme.warning.withValues(alpha: 0.1),
                  borderRadius: BorderRadius.circular(6),
                  border: Border.all(color: StudioTheme.warning.withValues(alpha: 0.2)),
                ),
                child: Text(
                  !backend.isConnected
                      ? 'Engine offline'
                      : 'Add API key in Settings',
                  style: TextStyle(
                    fontSize: 9,
                    color: StudioTheme.warning,
                    fontFamily: 'JetBrainsMono',
                  ),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  /// User message bubble — right-aligned feel with warm bg.
  Widget _buildUserBubble(ThemeData theme, ChatMessage msg) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 4, left: 16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
            decoration: BoxDecoration(
              color: const Color(0xFF3C3836),
              borderRadius: const BorderRadius.only(
                topLeft: Radius.circular(12),
                topRight: Radius.circular(12),
                bottomLeft: Radius.circular(12),
                bottomRight: Radius.circular(4),
              ),
            ),
            child: SelectionArea(
              child: Text(
                msg.content,
                style: theme.textTheme.bodyMedium!.copyWith(
                  fontSize: 11,
                  height: 1.4,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// Assistant message bubble — left-aligned with accent stripe.
  Widget _buildAssistantBubble(ThemeData theme, ChatMessage msg) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 6, right: 8),
      child: _HoverableBubble(
        onCopy: () {
          Clipboard.setData(ClipboardData(text: msg.content));
          ScaffoldMessenger.of(context).showSnackBar(
            const SnackBar(
              content: Text('Copied', style: TextStyle(fontSize: 11)),
              duration: Duration(seconds: 1),
            ),
          );
        },
        child: Container(
          decoration: BoxDecoration(
            color: const Color(0xFF242220),
            borderRadius: const BorderRadius.only(
              topLeft: Radius.circular(4),
              topRight: Radius.circular(12),
              bottomLeft: Radius.circular(12),
              bottomRight: Radius.circular(12),
            ),
            border: Border(
              left: BorderSide(
                color: theme.colorScheme.primary.withValues(alpha: 0.4),
                width: 2,
              ),
            ),
          ),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                SelectionArea(
                  child: MarkdownBody(
                    data: msg.content,
                    selectable: true,
                    styleSheet: MarkdownStyleSheet(
                      p: theme.textTheme.bodyMedium!.copyWith(fontSize: 11, height: 1.4),
                      code: theme.textTheme.bodyMedium!.copyWith(
                        fontSize: 10,
                        color: const Color(0xFFFED7AA),
                        backgroundColor: const Color(0xFF1A1816),
                      ),
                      codeblockDecoration: BoxDecoration(
                        color: const Color(0xFF1A1816),
                        borderRadius: BorderRadius.circular(6),
                        border: Border.all(color: const Color(0xFF3C3836)),
                      ),
                      codeblockPadding: const EdgeInsets.all(8),
                      blockquoteDecoration: BoxDecoration(
                        border: Border(
                          left: BorderSide(
                            color: theme.colorScheme.primary.withValues(alpha: 0.4),
                            width: 2,
                          ),
                        ),
                      ),
                      blockquotePadding: const EdgeInsets.only(left: 10, top: 4, bottom: 4),
                      h1: theme.textTheme.bodyMedium!.copyWith(fontSize: 13, fontWeight: FontWeight.w700),
                      h2: theme.textTheme.bodyMedium!.copyWith(fontSize: 12, fontWeight: FontWeight.w700),
                      h3: theme.textTheme.bodyMedium!.copyWith(fontSize: 11, fontWeight: FontWeight.w700),
                      listBullet: theme.textTheme.bodyMedium!.copyWith(fontSize: 11),
                      tableHead: theme.textTheme.bodyMedium!.copyWith(fontSize: 10, fontWeight: FontWeight.w700),
                      tableBody: theme.textTheme.bodyMedium!.copyWith(fontSize: 10),
                      tableBorder: TableBorder.all(color: const Color(0xFF3C3836)),
                      tableCellsPadding: const EdgeInsets.all(4),
                    ),
                  ),
                ),
                // Inline tile preview cards
                if (msg.previewImages.isNotEmpty) ...[
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: [
                      for (final img in msg.previewImages)
                        _buildTilePreviewCard(theme, img, msg.previewImages.length),
                    ],
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
    );
  }

  /// Status message — minimal centered chip.
  Widget _buildStatusChip(ThemeData theme, String content) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Center(
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
          decoration: BoxDecoration(
            color: theme.dividerColor.withValues(alpha: 0.2),
            borderRadius: BorderRadius.circular(8),
          ),
          child: Text(
            content,
            style: theme.textTheme.bodySmall!.copyWith(
              fontSize: 9,
              fontStyle: FontStyle.italic,
            ),
            textAlign: TextAlign.center,
          ),
        ),
      ),
    );
  }

  /// Typing indicator — three animated dots.
  Widget _buildTypingIndicator(ThemeData theme) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 6, right: 8),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
        decoration: BoxDecoration(
          color: const Color(0xFF242220),
          borderRadius: const BorderRadius.only(
            topLeft: Radius.circular(4),
            topRight: Radius.circular(12),
            bottomLeft: Radius.circular(12),
            bottomRight: Radius.circular(12),
          ),
          border: Border(
            left: BorderSide(
              color: theme.colorScheme.primary.withValues(alpha: 0.4),
              width: 2,
            ),
          ),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            SizedBox(
              width: 12, height: 12,
              child: CircularProgressIndicator(
                strokeWidth: 1.5,
                color: theme.colorScheme.primary.withValues(alpha: 0.6),
              ),
            ),
            const SizedBox(width: 8),
            Text(
              'Thinking...',
              style: theme.textTheme.bodySmall!.copyWith(
                fontSize: 10,
                color: theme.colorScheme.primary.withValues(alpha: 0.7),
                fontStyle: FontStyle.italic,
              ),
            ),
          ],
        ),
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
    final c = onTap != null ? color : color.withValues(alpha: 0.3);
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(4),
        hoverColor: c.withValues(alpha: 0.08),
        splashColor: c.withValues(alpha: 0.12),
        child: Padding(
          padding: const EdgeInsets.symmetric(vertical: 5),
          child: Center(
            child: Text(
              label,
              style: TextStyle(
                fontSize: 10,
                fontWeight: FontWeight.w500,
                color: c.withValues(alpha: 0.7),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Hoverable message bubble — shows a copy button on hover.
class _HoverableBubble extends StatefulWidget {
  const _HoverableBubble({required this.child, required this.onCopy});
  final Widget child;
  final VoidCallback onCopy;

  @override
  State<_HoverableBubble> createState() => _HoverableBubbleState();
}

class _HoverableBubbleState extends State<_HoverableBubble> {
  bool _hovering = false;

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      onEnter: (_) => setState(() => _hovering = true),
      onExit: (_) => setState(() => _hovering = false),
      child: Stack(
        clipBehavior: Clip.none,
        children: [
          widget.child,
          if (_hovering)
            Positioned(
              top: 4,
              right: 4,
              child: GestureDetector(
                onTap: widget.onCopy,
                child: Container(
                  padding: const EdgeInsets.all(3),
                  decoration: BoxDecoration(
                    color: const Color(0xFF3C3836),
                    borderRadius: BorderRadius.circular(4),
                    boxShadow: const [
                      BoxShadow(color: Color(0x33000000), blurRadius: 4, offset: Offset(0, 1)),
                    ],
                  ),
                  child: const Icon(Icons.copy, size: 10, color: Color(0xFFA8A29E)),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

/// Hoverable wrapper — shows a subtle "eye" overlay on hover to indicate
/// the tile image is clickable and will preview on canvas.
class _HoverablePreview extends StatefulWidget {
  const _HoverablePreview({
    required this.child,
    required this.onTap,
    this.onSecondaryTap,
  });
  final Widget child;
  final VoidCallback onTap;
  final VoidCallback? onSecondaryTap;

  @override
  State<_HoverablePreview> createState() => _HoverablePreviewState();
}

class _HoverablePreviewState extends State<_HoverablePreview> {
  bool _hovering = false;

  @override
  Widget build(BuildContext context) {
    return GestureDetector(
      onTap: widget.onTap,
      onSecondaryTap: widget.onSecondaryTap,
      child: MouseRegion(
        cursor: SystemMouseCursors.click,
        onEnter: (_) => setState(() => _hovering = true),
        onExit: (_) => setState(() => _hovering = false),
        child: Stack(
          children: [
            widget.child,
            if (_hovering)
              Positioned.fill(
                child: Container(
                  decoration: BoxDecoration(
                    color: Colors.white.withValues(alpha: 0.08),
                    borderRadius: BorderRadius.circular(4),
                  ),
                  child: const Center(
                    child: Icon(
                      Icons.visibility_outlined,
                      size: 18,
                      color: Color(0x66FFFFFF),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

/// Tiny icon button for tile preview card actions (12px icon).
class _TinyIconBtn extends StatelessWidget {
  const _TinyIconBtn({
    required this.icon,
    required this.tooltip,
    required this.onTap,
    this.isActive = false,
    this.color,
  });
  final IconData icon;
  final String tooltip;
  final VoidCallback onTap;
  final bool isActive;
  final Color? color;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final c = color ?? (isActive ? theme.colorScheme.primary : theme.textTheme.bodySmall?.color);
    return Tooltip(
      message: tooltip,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(3),
        child: Padding(
          padding: const EdgeInsets.all(3),
          child: Icon(icon, size: 12, color: c),
        ),
      ),
    );
  }
}

/// Small action button with icon + label for the pending bar.
class _SmallActionBtn extends StatelessWidget {
  const _SmallActionBtn({
    required this.icon,
    required this.label,
    this.onTap,
  });
  final IconData icon;
  final String label;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final c = onTap != null
        ? theme.textTheme.bodySmall?.color ?? Colors.grey
        : (theme.textTheme.bodySmall?.color ?? Colors.grey).withValues(alpha: 0.3);
    return InkWell(
      onTap: onTap,
      borderRadius: BorderRadius.circular(4),
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 11, color: c),
            const SizedBox(width: 3),
            Text(label, style: TextStyle(fontSize: 9, color: c)),
          ],
        ),
      ),
    );
  }
}

class _RejectChip extends StatelessWidget {
  const _RejectChip({
    required this.icon,
    required this.label,
    required this.onTap,
  });
  final IconData icon;
  final String label;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(8),
        hoverColor: StudioTheme.error.withValues(alpha: 0.08),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: theme.dividerColor.withValues(alpha: 0.2),
            ),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 12, color: theme.colorScheme.onSurface.withValues(alpha: 0.5)),
              const SizedBox(width: 5),
              Text(
                label,
                style: theme.textTheme.bodySmall!.copyWith(
                  fontSize: 10,
                  color: theme.colorScheme.onSurface.withValues(alpha: 0.7),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}


