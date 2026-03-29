import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../models/pixel_canvas.dart' show CanvasSize, EditorMode;
import '../models/palette.dart';
import '../providers/backend_provider.dart';
import '../providers/canvas_provider.dart';
import '../providers/claude_provider.dart';
import '../providers/palette_provider.dart';
import '../providers/tab_provider.dart';
import '../providers/tilemap_provider.dart';
import '../services/llm_provider.dart';
import '../services/export_service.dart';
import '../theme/studio_theme.dart';
import 'adapter_picker.dart';
import 'llm_provider_settings.dart';
import 'scanner_dialog.dart';

const _themes = [
  ('dark_fantasy', 'Dark Fantasy', Icons.castle),
  ('light_fantasy', 'Light Fantasy', Icons.wb_sunny),
  ('sci_fi', 'Sci-Fi', Icons.rocket_launch),
  ('nature', 'Nature', Icons.park),
  ('gameboy', 'Game Boy', Icons.gamepad),
  ('nes', 'Retro 8-bit', Icons.sports_esports),
];

/// Sync [paletteProvider] from a loadSource engine response.
/// Returns true if the palette was successfully synced.
bool _syncPaletteFromResponse(WidgetRef ref, Map<String, dynamic> resp) {
  final palette = PixlPalette.fromEngineResponse(resp);
  if (palette != null) {
    ref.read(paletteProvider.notifier).setPalette(palette);
    return true;
  }
  return false;
}

Future<void> showNewProjectDialog(BuildContext context, WidgetRef ref) async {
  final result = await showDialog<_NewProjectResult>(
    context: context,
    builder: (ctx) => const _NewProjectDialog(),
  );
  if (result == null) return;

  // Resolve palette for the selected theme.
  PixlPalette? palette;
  final themeEntry = _themes.where((t) => t.$1 == result.themeId);
  if (themeEntry.isNotEmpty) {
    final match = BuiltInPalettes.all.where((p) => p.name == themeEntry.first.$2);
    if (match.isNotEmpty) palette = match.first;
  }

  // Create a new tab with the selected settings.
  ref.read(tabManagerProvider.notifier).newTab(
    name: result.name,
    canvasSize: result.canvasSize,
    palette: palette,
  );

  // Blank canvas — done.
  if (result.themeId == null) return;

  // Ensure engine is connected.
  final backend = ref.read(backendProvider);
  if (!backend.isConnected) {
    final service = ref.read(claudeProvider.notifier).service;
    final isLocal = service.provider == LlmProviderType.pixlLocal;
    await ref.read(backendProvider.notifier).connect(
      model: isLocal ? service.pixlModel : null,
      adapter: isLocal && service.hasPixlAdapter ? service.pixlAdapter : null,
    );
  }

  // Load theme template into engine.
  final tmplResp = await ref.read(backendProvider.notifier).backend.newFromTemplate(result.themeId!);
  if (tmplResp.containsKey('source')) {
    final loadResp = await ref.read(backendProvider.notifier).loadSource(tmplResp['source'] as String);
    // Sync palette from engine data — works for all themes, not just built-ins.
    // Falls back to the resolved built-in palette if the engine doesn't return colors.
    if (!_syncPaletteFromResponse(ref, loadResp) && palette != null) {
      ref.read(paletteProvider.notifier).setPalette(palette);
    }
  } else if (palette != null) {
    ref.read(paletteProvider.notifier).setPalette(palette);
  }
}

class _NewProjectResult {
  const _NewProjectResult({
    required this.name,
    required this.canvasSize,
    this.themeId,
  });
  final String name;
  final CanvasSize canvasSize;
  final String? themeId; // null = blank canvas
}

class _NewProjectDialog extends StatefulWidget {
  const _NewProjectDialog();

  @override
  State<_NewProjectDialog> createState() => _NewProjectDialogState();
}

class _NewProjectDialogState extends State<_NewProjectDialog> {
  final _nameController = TextEditingController(text: 'untitled');
  CanvasSize _canvasSize = CanvasSize.s16x16;
  String? _selectedTheme = 'dark_fantasy';

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final t = Theme.of(context);
    final labelStyle = t.textTheme.bodySmall!.copyWith(
      fontWeight: FontWeight.w600,
      fontSize: 11,
      letterSpacing: 0.5,
    );

    return Dialog(
      backgroundColor: t.cardColor,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      child: Container(
        width: 360,
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // ── Header ──
            Text('New Project', style: t.textTheme.bodyMedium!.copyWith(
              fontSize: 16, fontWeight: FontWeight.w700,
            )),
            const SizedBox(height: 16),

            // ── Name ──
            Text('NAME', style: labelStyle),
            const SizedBox(height: 4),
            TextField(
              controller: _nameController,
              style: t.textTheme.bodyMedium!.copyWith(fontSize: 12),
              decoration: InputDecoration(
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(horizontal: 10, vertical: 8),
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(4),
                  borderSide: BorderSide(color: t.dividerColor),
                ),
                focusedBorder: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(4),
                  borderSide: BorderSide(color: t.colorScheme.primary),
                ),
              ),
            ),
            const SizedBox(height: 14),

            // ── Canvas Size ──
            Text('CANVAS SIZE', style: labelStyle),
            const SizedBox(height: 6),
            Wrap(
              spacing: 6,
              runSpacing: 6,
              children: CanvasSize.values.map((size) {
                final isActive = size == _canvasSize;
                return InkWell(
                  onTap: () => setState(() => _canvasSize = size),
                  borderRadius: BorderRadius.circular(4),
                  child: Container(
                    padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 6),
                    decoration: BoxDecoration(
                      color: isActive ? t.colorScheme.primary.withValues(alpha: 0.3) : null,
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                        color: isActive ? t.colorScheme.primary : t.dividerColor,
                      ),
                    ),
                    child: Text(
                      size.label,
                      style: t.textTheme.bodySmall!.copyWith(
                        fontSize: 11,
                        color: isActive ? t.colorScheme.primary : null,
                        fontWeight: isActive ? FontWeight.w700 : null,
                      ),
                    ),
                  ),
                );
              }).toList(),
            ),
            const SizedBox(height: 14),

            // ── Theme / Palette ──
            Text('THEME', style: labelStyle),
            const SizedBox(height: 6),
            ..._themes.map((entry) {
              final (id, label, icon) = entry;
              final isActive = id == _selectedTheme;
              return Padding(
                padding: const EdgeInsets.only(bottom: 4),
                child: InkWell(
                  onTap: () => setState(() => _selectedTheme = id),
                  borderRadius: BorderRadius.circular(6),
                  child: Container(
                    padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7),
                    decoration: BoxDecoration(
                      color: isActive ? t.colorScheme.primary.withValues(alpha: 0.15) : null,
                      borderRadius: BorderRadius.circular(6),
                      border: Border.all(
                        color: isActive ? t.colorScheme.primary : t.dividerColor,
                        width: isActive ? 1.5 : 1,
                      ),
                    ),
                    child: Row(
                      children: [
                        Icon(icon, size: 16, color: isActive ? t.colorScheme.primary : t.textTheme.bodySmall?.color),
                        const SizedBox(width: 10),
                        Text(label, style: t.textTheme.bodySmall!.copyWith(
                          fontSize: 12,
                          color: isActive ? t.colorScheme.primary : null,
                          fontWeight: isActive ? FontWeight.w600 : null,
                        )),
                      ],
                    ),
                  ),
                ),
              );
            }),
            // Blank canvas option
            Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: InkWell(
                onTap: () => setState(() => _selectedTheme = null),
                borderRadius: BorderRadius.circular(6),
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 7),
                  decoration: BoxDecoration(
                    color: _selectedTheme == null ? t.colorScheme.primary.withValues(alpha: 0.15) : null,
                    borderRadius: BorderRadius.circular(6),
                    border: Border.all(
                      color: _selectedTheme == null ? t.colorScheme.primary : t.dividerColor,
                      width: _selectedTheme == null ? 1.5 : 1,
                    ),
                  ),
                  child: Row(
                    children: [
                      Icon(Icons.crop_square, size: 16, color: _selectedTheme == null ? t.colorScheme.primary : t.textTheme.bodySmall?.color),
                      const SizedBox(width: 10),
                      Text('Blank Canvas', style: t.textTheme.bodySmall!.copyWith(
                        fontSize: 12,
                        color: _selectedTheme == null ? t.colorScheme.primary : null,
                        fontWeight: _selectedTheme == null ? FontWeight.w600 : null,
                      )),
                    ],
                  ),
                ),
              ),
            ),
            const SizedBox(height: 16),

            // ── Actions ──
            Row(
              mainAxisAlignment: MainAxisAlignment.end,
              children: [
                TextButton(
                  onPressed: () => Navigator.pop(context),
                  child: Text('Cancel', style: t.textTheme.bodySmall),
                ),
                const SizedBox(width: 8),
                FilledButton(
                  onPressed: () {
                    final name = _nameController.text.trim();
                    Navigator.pop(context, _NewProjectResult(
                      name: name.isEmpty ? 'untitled' : name,
                      canvasSize: _canvasSize,
                      themeId: _selectedTheme,
                    ));
                  },
                  child: const Text('Create', style: TextStyle(fontSize: 12)),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

Future<void> openPaxFile(BuildContext context, WidgetRef ref) async {
  final messenger = ScaffoldMessenger.of(context);

  // Use FileType.any because .pax isn't a registered UTI on macOS —
  // FileType.custom with allowedExtensions silently shows no files.
  final result = await FilePicker.platform.pickFiles(
    dialogTitle: 'Open PAX File',
    type: FileType.any,
  );
  if (result == null || result.files.isEmpty) return;

  final path = result.files.single.path;
  if (path == null) return;

  if (!path.endsWith('.pax') && !path.endsWith('.pixl')) {
    messenger.showSnackBar(const SnackBar(
      content: Text('Please select a .pax or .pixl file'),
    ));
    return;
  }

  final source = await File(path).readAsString();
  await ExportService.setLastFilePath(path);
  final backend = ref.read(backendProvider);

  // If engine isn't connected, start it with this file
  if (!backend.isConnected) {
    messenger.showSnackBar(SnackBar(
      content: Text('Starting engine with ${path.split('/').last}...'),
      duration: const Duration(seconds: 4),
    ));
    final service = ref.read(claudeProvider.notifier).service;
    final isLocal = service.provider == LlmProviderType.pixlLocal;
    await ref.read(backendProvider.notifier).connect(
      paxFile: path,
      model: isLocal ? service.pixlModel : null,
      adapter: isLocal && service.hasPixlAdapter ? service.pixlAdapter : null,
    );

    if (!ref.read(backendProvider).isConnected) {
      messenger.showSnackBar(const SnackBar(
        content: Text(
          'Could not start engine. Make sure pixl is built:\n'
          'cd tool && cargo build --release',
        ),
        duration: Duration(seconds: 6),
      ));
      return;
    }

    // Engine started with the file — load again to get palette info back.
    final resp = await ref.read(backendProvider.notifier).loadSource(source);
    _syncPaletteFromResponse(ref, resp);

    messenger.showSnackBar(SnackBar(
      content: Text('Loaded ${path.split('/').last}'),
      duration: const Duration(seconds: 2),
    ));
    return;
  }

  // Engine already running — load source into session
  final resp = await ref.read(backendProvider.notifier).loadSource(source);
  if (resp.containsKey('error')) {
    messenger.showSnackBar(SnackBar(
      content: Text('Failed to load: ${resp['error']}'),
    ));
  } else {
    _syncPaletteFromResponse(ref, resp);
    messenger.showSnackBar(SnackBar(
      content: Text('Loaded ${path.split('/').last}'),
      duration: const Duration(seconds: 2),
    ));
  }
}

/// Top toolbar with logo, mode toggle, and canvas controls.
class TopBar extends ConsumerWidget {
  const TopBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cs = ref.watch(canvasProvider);
    final notifier = ref.read(canvasProvider.notifier);
    final theme = Theme.of(context);

    return Container(
      height: 40,
      decoration: StudioTheme.topBarDecoration,
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: Row(
        children: [
          // Logo
          Text(
            'PIXL',
            style: theme.textTheme.bodyMedium!.copyWith(
              fontWeight: FontWeight.w700,
              fontSize: 15,
              color: theme.colorScheme.primary,
              letterSpacing: 2,
            ),
          ),
          Text(
            ' STUDIO',
            style: theme.textTheme.bodySmall!.copyWith(
              fontSize: 11,
              letterSpacing: 1.5,
            ),
          ),
          const SizedBox(width: 16),

          // Mode toggle
          _ModeToggle(),

          const Spacer(),

          // Style Scanner
          Tooltip(
            message: 'Style Scanner — train from reference art',
            child: InkWell(
              onTap: () => ScannerDialog.show(context),
              borderRadius: BorderRadius.circular(4),
              child: Padding(
                padding: const EdgeInsets.all(4),
                child: Icon(Icons.document_scanner, size: 16,
                  color: theme.colorScheme.onSurface.withValues(alpha: 0.7)),
              ),
            ),
          ),
          const SizedBox(width: 4),

          // Adapter picker
          const AdapterPicker(),
          const SizedBox(width: 8),

          // API key status
          _ApiKeyBadge(),
          const SizedBox(width: 12),

          // Undo/Redo
          _BarButton(
            label: 'Undo (Cmd+Z)',
            icon: Icons.undo,
            onTap: notifier.canUndo ? () => notifier.undo() : null,
          ),
          _BarButton(
            label: 'Redo (Cmd+Shift+Z)',
            icon: Icons.redo,
            onTap: notifier.canRedo ? () => notifier.redo() : null,
          ),
          const SizedBox(width: 4),

          // Zoom controls
          _BarButton(
            label: 'Zoom Out',
            icon: Icons.remove,
            onTap: () => notifier.zoomOut(),
          ),
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 4),
            child: Text(
              '${cs.zoomLevel.round()}x',
              style: theme.textTheme.bodySmall,
            ),
          ),
          _BarButton(
            label: 'Zoom In',
            icon: Icons.add,
            onTap: () => notifier.zoomIn(),
          ),
        ],
      ),
    );
  }
}

class _ModeToggle extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final mode = ref.watch(editorModeProvider);
    final theme = Theme.of(context);

    Widget modeButton(EditorMode m, IconData icon, String label) {
      final isActive = mode == m;
      return InkWell(
        onTap: () => ref.read(editorModeProvider.notifier).state = m,
        borderRadius: BorderRadius.circular(4),
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
          decoration: BoxDecoration(
            color: isActive ? theme.colorScheme.primary.withValues(alpha: 0.2) : null,
            borderRadius: BorderRadius.circular(4),
          ),
          child: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 14,
                color: isActive ? theme.colorScheme.primary : theme.textTheme.bodySmall?.color),
              const SizedBox(width: 4),
              Text(label, style: TextStyle(
                fontSize: 10,
                fontWeight: isActive ? FontWeight.w700 : null,
                color: isActive ? theme.colorScheme.primary : theme.textTheme.bodySmall?.color,
              )),
            ],
          ),
        ),
      );
    }

    return Container(
      padding: const EdgeInsets.all(2),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(6),
        border: Border.all(color: theme.dividerColor),
      ),
      child: Row(
        mainAxisSize: MainAxisSize.min,
        children: [
          modeButton(EditorMode.pixel, Icons.edit, 'Pixel'),
          modeButton(EditorMode.tilemap, Icons.grid_view, 'Tilemap'),
          modeButton(EditorMode.backdrop, Icons.landscape, 'Backdrop'),
          modeButton(EditorMode.composite, Icons.dashboard, 'Composite'),
        ],
      ),
    );
  }
}

class _ApiKeyBadge extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final llm = ref.watch(claudeProvider);
    final theme = Theme.of(context);
    if (llm.hasApiKey) {
      // Show active provider name
      final providerShort = switch (llm.provider) {
        LlmProviderType.anthropic => 'Claude',
        LlmProviderType.openai => 'GPT',
        LlmProviderType.gemini => 'Gemini',
        LlmProviderType.ollama => 'Ollama',
        LlmProviderType.pixlLocal => 'LoRA',
      };
      return InkWell(
        onTap: () => LlmProviderSettings.show(context),
        borderRadius: BorderRadius.circular(4),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.key, size: 12, color: Color(0xFF4caf50)),
            const SizedBox(width: 4),
            Text(providerShort, style: theme.textTheme.bodySmall!.copyWith(
              fontSize: 10, color: StudioTheme.success,
            )),
          ],
        ),
      );
    }
    return InkWell(
      onTap: () => LlmProviderSettings.show(context),
      borderRadius: BorderRadius.circular(4),
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(4),
          border: Border.all(color: StudioTheme.warning),
        ),
        child: Text('Add API Key', style: theme.textTheme.bodySmall!.copyWith(
          fontSize: 9, color: StudioTheme.warning,
        )),
      ),
    );
  }
}

class _BarButton extends StatelessWidget {
  const _BarButton({
    required this.label,
    required this.icon,
    this.onTap,
    this.active = false,
  });

  final String label;
  final IconData icon;
  final VoidCallback? onTap;
  final bool active;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Tooltip(
      message: label,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(4),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 6),
          child: Icon(
            icon,
            size: 18,
            color: onTap == null
                ? theme.disabledColor
                : active
                    ? theme.colorScheme.primary
                    : null,
          ),
        ),
      ),
    );
  }
}
