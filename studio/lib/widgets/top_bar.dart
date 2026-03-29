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
import 'llm_provider_settings.dart';
import 'settings_dialog.dart';
import 'shortcuts_dialog.dart';
import 'tilegroup_dialog.dart';
import 'backdrop_dialog.dart';
import 'convert_dialog.dart';
import 'training_dialog.dart';
import 'wfc_dialog.dart';

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

Future<void> _showNewProjectDialog(BuildContext context, WidgetRef ref) async {
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

Future<void> _openPaxFile(BuildContext context, WidgetRef ref) async {
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

class _ModeToggleXDELETEME
  });

  final String label;
  final List<PopupMenuEntry<String>> items;
  final void Function(String value) onSelected;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return PopupMenuButton<String>(
      tooltip: label,
      onSelected: onSelected,
      offset: const Offset(0, 36),
      itemBuilder: (_) => items,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 8),
        child: Text(
          label,
          style: theme.textTheme.bodySmall!.copyWith(
            fontSize: 12,
            fontWeight: FontWeight.w500,
          ),
        ),
      ),
    );
  }
}

PopupMenuItem<String> _menuItem(String value, String label, {IconData? icon, String? shortcut}) {
  return PopupMenuItem<String>(
    value: value,
    child: Row(
      children: [
        if (icon != null) ...[
          Icon(icon, size: 15),
          const SizedBox(width: 8),
        ],
        Expanded(child: Text(label, style: const TextStyle(fontSize: 12))),
        if (shortcut != null) ...[
          const SizedBox(width: 16),
          Text(shortcut, style: const TextStyle(fontSize: 10, color: Colors.grey)),
        ],
      ],
    ),
  );
}

// ── File Menu ──

class _FileMenu extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return _MenuButton(
      label: 'File',
      items: [
        _menuItem('new', 'New Project...', icon: Icons.add, shortcut: '\u2318N'),
        _menuItem('open', 'Open PAX...', icon: Icons.folder_open, shortcut: '\u2318O'),
        _menuItem('recent', 'Open Recent...', icon: Icons.history),
        const PopupMenuDivider(),
        _menuItem('save', 'Save', icon: Icons.save, shortcut: '\u2318S'),
        _menuItem('save_as', 'Save As...', icon: Icons.save_as, shortcut: '\u21e7\u2318S'),
        const PopupMenuDivider(),
        _menuItem('close_tab', 'Close Tab', icon: Icons.close, shortcut: '\u2318W'),
        const PopupMenuDivider(),
        _menuItem('settings', 'Settings...', icon: Icons.settings),
        _menuItem('llm', 'LLM Provider...', icon: Icons.smart_toy),
      ],
      onSelected: (value) async {
        final messenger = ScaffoldMessenger.of(context);
        switch (value) {
          case 'new':
            _showNewProjectDialog(context, ref);
            break;
          case 'open':
            _openPaxFile(context, ref);
            break;
          case 'recent':
            _showRecentFilesDialog(context, ref);
            break;
          case 'save':
            final source = await ref.read(backendProvider.notifier).getPaxSource();
            if (source == null) return;
            final ok = await ExportService.quickSavePax(source);
            if (!ok) await ExportService.savePaxSource(source);
            break;
          case 'save_as':
            final source = await ref.read(backendProvider.notifier).getPaxSource();
            if (source != null) {
              final ok = await ExportService.savePaxSource(source);
              messenger.showSnackBar(SnackBar(
                content: Text(ok ? 'PAX source saved' : 'Save cancelled'),
                duration: const Duration(seconds: 2),
              ));
            } else {
              messenger.showSnackBar(const SnackBar(
                content: Text('No PAX source available (engine not connected?)'),
              ));
            }
            break;
          case 'close_tab':
            final tabs = ref.read(tabManagerProvider);
            if (tabs.activeTabId != null) {
              ref.read(tabManagerProvider.notifier).closeTab(tabs.activeTabId!);
            }
            break;
          case 'settings':
            SettingsDialog.show(context);
            break;
          case 'llm':
            LlmProviderSettings.show(context);
            break;
        }
      },
    );
  }

}

Future<void> _showRecentFilesDialog(BuildContext context, WidgetRef ref) async {
  final files = await ExportService.getRecentFiles();
  if (files.isEmpty || !context.mounted) return;
  final path = await showDialog<String>(
    context: context,
    builder: (ctx) {
      final theme = Theme.of(ctx);
      return Dialog(
        backgroundColor: theme.cardColor,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
        child: Container(
          width: 320,
          padding: const EdgeInsets.all(16),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Text('Recent Files', style: theme.textTheme.bodyMedium!.copyWith(
                fontSize: 14, fontWeight: FontWeight.w700,
              )),
              const SizedBox(height: 12),
              ...files.map((f) {
                final name = f.split('/').last;
                return InkWell(
                  onTap: () => Navigator.pop(ctx, f),
                  borderRadius: BorderRadius.circular(4),
                  child: Padding(
                    padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
                    child: Row(
                      children: [
                        Icon(Icons.description, size: 14, color: theme.colorScheme.primary),
                        const SizedBox(width: 8),
                        Expanded(child: Text(name, style: theme.textTheme.bodySmall!.copyWith(fontSize: 12))),
                      ],
                    ),
                  ),
                );
              }),
              const SizedBox(height: 8),
              Align(
                alignment: Alignment.centerRight,
                child: TextButton(
                  onPressed: () => Navigator.pop(ctx),
                  child: Text('Cancel', style: theme.textTheme.bodySmall),
                ),
              ),
            ],
          ),
        ),
      );
    },
  );
  if (path != null && context.mounted) {
    await _openRecentFile(context, ref, path);
  }
}

Future<void> _openRecentFile(BuildContext context, WidgetRef ref, String path) async {
  final file = File(path);
  if (!await file.exists()) return;
  final source = await file.readAsString();
  final backend = ref.read(backendProvider);

  if (!backend.isConnected) {
    final service = ref.read(claudeProvider.notifier).service;
    final isLocal = service.provider == LlmProviderType.pixlLocal;
    await ref.read(backendProvider.notifier).connect(
      paxFile: path,
      model: isLocal ? service.pixlModel : null,
      adapter: isLocal && service.hasPixlAdapter ? service.pixlAdapter : null,
    );
    final resp = await ref.read(backendProvider.notifier).loadSource(source);
    _syncPaletteFromResponse(ref, resp);
  } else {
    final resp = await ref.read(backendProvider.notifier).loadSource(source);
    _syncPaletteFromResponse(ref, resp);
  }
  await ExportService.setLastFilePath(path);
}

// ── Tools Menu ──

class _ToolsMenu extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return _MenuButton(
      label: 'Tools',
      items: [
        _menuItem('tilegroup', 'Generate Tilegroup', icon: Icons.grid_view),
        _menuItem('wfc', 'WFC Map', icon: Icons.auto_awesome_mosaic),
        _menuItem('convert', 'Convert to Pixel Art', icon: Icons.auto_fix_high),
        const PopupMenuDivider(),
        _menuItem('backdrop', 'Backdrop', icon: Icons.landscape),
        _menuItem('training', 'Training', icon: Icons.model_training),
      ],
      onSelected: (value) {
        switch (value) {
          case 'tilegroup':
            TilegroupDialog.show(context);
            break;
          case 'wfc':
            WfcDialog.show(context);
            break;
          case 'convert':
            ConvertDialog.show(context);
            break;
          case 'backdrop':
            BackdropDialog.show(context);
            break;
          case 'training':
            TrainingDialog.show(context);
            break;
        }
      },
    );
  }
}

// ── View Menu ──

class _ViewMenu extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final cs = ref.watch(canvasProvider);
    final notifier = ref.read(canvasProvider.notifier);

    return _MenuButton(
      label: 'View',
      items: [
        _menuItem('grid', cs.showGrid ? 'Hide Grid' : 'Show Grid',
            icon: cs.showGrid ? Icons.grid_on : Icons.grid_off),
        _menuItem('blueprint', 'Blueprint Guide', icon: Icons.accessibility_new),
        const PopupMenuDivider(),
        _menuItem('zoom_in', 'Zoom In', icon: Icons.add, shortcut: '\u2318+'),
        _menuItem('zoom_out', 'Zoom Out', icon: Icons.remove, shortcut: '\u2318\u2212'),
        _menuItem('zoom_reset', 'Reset Zoom', icon: Icons.fit_screen),
      ],
      onSelected: (value) {
        switch (value) {
          case 'grid':
            notifier.toggleGrid();
            break;
          case 'blueprint':
            _toggleBlueprint(context, ref);
            break;
          case 'zoom_in':
            notifier.zoomIn();
            break;
          case 'zoom_out':
            notifier.zoomOut();
            break;
          case 'zoom_reset':
            notifier.resetZoom();
            break;
        }
      },
    );
  }

  Future<void> _toggleBlueprint(BuildContext context, WidgetRef ref) async {
    final bp = ref.read(blueprintProvider);
    if (bp != null) {
      ref.read(blueprintProvider.notifier).state = null;
    } else {
      final cs = ref.read(canvasProvider);
      final resp = await ref.read(backendProvider.notifier).backend.getBlueprint(
        width: cs.width,
        height: cs.height,
      );
      final landmarks = resp['landmarks'] as List<dynamic>?;
      if (landmarks != null) {
        ref.read(blueprintProvider.notifier).state =
            landmarks.map((l) => Map<String, dynamic>.from(l as Map)).toList();
      }
    }
  }
}

// ── Help Menu ──

class _HelpMenu extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return _MenuButton(
      label: 'Help',
      items: [
        _menuItem('shortcuts', 'Keyboard Shortcuts', icon: Icons.keyboard, shortcut: '\u2318/'),
      ],
      onSelected: (value) {
        switch (value) {
          case 'shortcuts':
            ShortcutsDialog.show(context);
            break;
        }
      },
    );
  }
}

class _ExportMenu extends ConsumerWidget {
  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return _MenuButton(
      label: 'Export',
      items: [
        _menuItem('png', 'Export PNG (4x)', icon: Icons.image),
        _menuItem('png8x', 'Export PNG (8x)', icon: Icons.image),
        const PopupMenuDivider(),
        _menuItem('atlas', 'Export Atlas', icon: Icons.grid_4x4),
        const PopupMenuDivider(),
        _menuItem('tiled', 'Export for Tiled', icon: Icons.map),
        _menuItem('godot', 'Export for Godot', icon: Icons.videogame_asset),
        _menuItem('texturepacker', 'Export for TexturePacker', icon: Icons.texture),
        _menuItem('gbstudio', 'Export for GB Studio', icon: Icons.gamepad),
        _menuItem('unity', 'Export for Unity', icon: Icons.sports_esports),
      ],
      onSelected: (value) async {
        final messenger = ScaffoldMessenger.of(context);
        switch (value) {
          case 'png':
            final cs = ref.read(canvasProvider);
            final ok = await ExportService.exportCanvasPng(canvasState: cs, scale: 4);
            messenger.showSnackBar(SnackBar(
              content: Text(ok ? 'PNG exported' : 'Export cancelled'),
              duration: const Duration(seconds: 2),
            ));
            break;
          case 'png8x':
            final cs = ref.read(canvasProvider);
            final ok = await ExportService.exportCanvasPng(canvasState: cs, scale: 8);
            messenger.showSnackBar(SnackBar(
              content: Text(ok ? 'PNG exported (8x)' : 'Export cancelled'),
              duration: const Duration(seconds: 2),
            ));
            break;
          case 'atlas':
            final resp = await ref.read(backendProvider.notifier).packAtlas();
            final png = resp['png'] as String?;
            if (png != null) {
              final ok = await ExportService.saveAtlasPng(png);
              messenger.showSnackBar(SnackBar(
                content: Text(ok ? 'Atlas exported' : 'Export cancelled'),
                duration: const Duration(seconds: 2),
              ));
            } else {
              messenger.showSnackBar(SnackBar(
                content: Text('Atlas pack failed: ${resp['error'] ?? 'unknown'}'),
              ));
            }
            break;
          case 'tiled':
          case 'godot':
          case 'texturepacker':
          case 'gbstudio':
          case 'unity':
            final dir = await FilePicker.platform.getDirectoryPath(
              dialogTitle: 'Export to $value',
            );
            if (dir == null) {
              messenger.showSnackBar(const SnackBar(
                content: Text('Export cancelled'),
                duration: Duration(seconds: 2),
              ));
              break;
            }
            final resp = await ref.read(backendProvider.notifier).backend.exportToEngine(
              format: value,
              outDir: dir,
            );
            if (resp['ok'] == true) {
              final files = resp['files'] as int? ?? 0;
              messenger.showSnackBar(SnackBar(
                content: Text('Exported $files files to $dir'),
                duration: const Duration(seconds: 3),
              ));
            } else {
              messenger.showSnackBar(SnackBar(
                content: Text('Export failed: ${resp['error'] ?? 'unknown'}'),
              ));
            }
            break;
        }
      },
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
