import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'models/palette.dart';
import 'providers/backend_provider.dart';
import 'providers/canvas_provider.dart';
import 'providers/claude_provider.dart';
import 'providers/palette_provider.dart';
import 'providers/tab_provider.dart';
import 'services/export_service.dart';
import 'services/llm_provider.dart';
import 'theme/studio_theme.dart';
import 'widgets/backdrop_dialog.dart';
import 'widgets/convert_dialog.dart';
import 'widgets/export_dialog.dart';
import 'widgets/llm_provider_settings.dart';
import 'widgets/settings_dialog.dart';
import 'widgets/shortcuts_dialog.dart';
import 'widgets/studio_shell.dart';
import 'widgets/tilegroup_dialog.dart';
import 'widgets/top_bar.dart';
import 'widgets/training_dialog.dart';
import 'widgets/wfc_dialog.dart';

final _navigatorKey = GlobalKey<NavigatorState>();

void main() {
  runApp(const ProviderScope(child: PixlStudioApp()));
}

class PixlStudioApp extends ConsumerStatefulWidget {
  const PixlStudioApp({super.key});

  @override
  ConsumerState<PixlStudioApp> createState() => _PixlStudioAppState();
}

class _PixlStudioAppState extends ConsumerState<PixlStudioApp> {
  List<String> _recentFiles = [];

  @override
  void initState() {
    super.initState();
    _refreshRecentFiles();
  }

  Future<void> _refreshRecentFiles() async {
    final files = await ExportService.getRecentFiles();
    if (mounted) {
      setState(() => _recentFiles = files.take(15).toList());
    }
  }

  @override
  Widget build(BuildContext context) {
    return PlatformMenuBar(
      menus: _buildMenus(),
      child: MaterialApp(
        navigatorKey: _navigatorKey,
        title: 'PIXL Studio',
        debugShowCheckedModeBanner: false,
        theme: StudioTheme.theme,
        home: const StudioShell(),
      ),
    );
  }

  /// Get the navigator context for showing dialogs.
  /// Falls back to widget context if navigator isn't ready yet.
  BuildContext get _ctx =>
      _navigatorKey.currentContext ?? context;

  List<PlatformMenu> _buildMenus() {
    return [
      // ── App menu (macOS app name menu) ──
      PlatformMenu(
        label: 'PIXL Studio',
        menus: [
          const PlatformProvidedMenuItem(type: PlatformProvidedMenuItemType.about),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Settings...',
                shortcut: const SingleActivator(LogicalKeyboardKey.comma, meta: true),
                onSelected: () => SettingsDialog.show(_ctx),
              ),
              PlatformMenuItem(
                label: 'LLM Provider...',
                onSelected: () => LlmProviderSettings.show(_ctx),
              ),
            ],
          ),
          const PlatformProvidedMenuItem(type: PlatformProvidedMenuItemType.quit),
        ],
      ),

      // ── File ──
      PlatformMenu(
        label: 'File',
        menus: [
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'New Project...',
                shortcut: const SingleActivator(LogicalKeyboardKey.keyN, meta: true),
                onSelected: () => showNewProjectDialog(_ctx, ref),
              ),
              PlatformMenuItem(
                label: 'Open PAX...',
                shortcut: const SingleActivator(LogicalKeyboardKey.keyO, meta: true),
                onSelected: () => openPaxFile(_ctx, ref),
              ),
              if (_recentFiles.isNotEmpty)
                PlatformMenu(
                  label: 'Open Recent',
                  menus: _recentFiles.map((path) {
                    final name = path.split('/').last;
                    return PlatformMenuItem(
                      label: name,
                      onSelected: () => _openRecentFile(path),
                    );
                  }).toList(),
                ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Save',
                shortcut: const SingleActivator(LogicalKeyboardKey.keyS, meta: true),
                onSelected: () => _save(),
              ),
              PlatformMenuItem(
                label: 'Save As...',
                shortcut: const SingleActivator(LogicalKeyboardKey.keyS, meta: true, shift: true),
                onSelected: () => _saveAs(),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Export...',
                shortcut: const SingleActivator(LogicalKeyboardKey.keyE, meta: true, shift: true),
                onSelected: () => ExportDialog.show(_ctx),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Close Tab',
                shortcut: const SingleActivator(LogicalKeyboardKey.keyW, meta: true),
                onSelected: () {
                  final tabs = ref.read(tabManagerProvider);
                  if (tabs.activeTabId != null) {
                    ref.read(tabManagerProvider.notifier).closeTab(tabs.activeTabId!);
                  }
                },
              ),
            ],
          ),
        ],
      ),

      // ── Edit ──
      PlatformMenu(
        label: 'Edit',
        menus: [
          PlatformMenuItem(
            label: 'Undo',
            shortcut: const SingleActivator(LogicalKeyboardKey.keyZ, meta: true),
            onSelected: () {
              final notifier = ref.read(canvasProvider.notifier);
              if (notifier.canUndo) notifier.undo();
            },
          ),
          PlatformMenuItem(
            label: 'Redo',
            shortcut: const SingleActivator(LogicalKeyboardKey.keyZ, meta: true, shift: true),
            onSelected: () {
              final notifier = ref.read(canvasProvider.notifier);
              if (notifier.canRedo) notifier.redo();
            },
          ),
        ],
      ),

      // ── Tools ──
      PlatformMenu(
        label: 'Tools',
        menus: [
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Generate Tilegroup...',
                onSelected: () => TilegroupDialog.show(_ctx),
              ),
              PlatformMenuItem(
                label: 'WFC Map...',
                onSelected: () => WfcDialog.show(_ctx),
              ),
              PlatformMenuItem(
                label: 'Convert to Pixel Art...',
                onSelected: () => ConvertDialog.show(_ctx),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Backdrop...',
                onSelected: () => BackdropDialog.show(_ctx),
              ),
              PlatformMenuItem(
                label: 'Training...',
                onSelected: () => TrainingDialog.show(_ctx),
              ),
            ],
          ),
        ],
      ),

      // ── View ──
      PlatformMenu(
        label: 'View',
        menus: [
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Toggle Grid',
                shortcut: const SingleActivator(LogicalKeyboardKey.keyG, meta: true),
                onSelected: () => ref.read(canvasProvider.notifier).toggleGrid(),
              ),
              PlatformMenuItem(
                label: 'Blueprint Guide',
                onSelected: () => _toggleBlueprint(),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Zoom In',
                shortcut: const SingleActivator(LogicalKeyboardKey.equal, meta: true),
                onSelected: () => ref.read(canvasProvider.notifier).zoomIn(),
              ),
              PlatformMenuItem(
                label: 'Zoom Out',
                shortcut: const SingleActivator(LogicalKeyboardKey.minus, meta: true),
                onSelected: () => ref.read(canvasProvider.notifier).zoomOut(),
              ),
              PlatformMenuItem(
                label: 'Reset Zoom',
                shortcut: const SingleActivator(LogicalKeyboardKey.digit0, meta: true),
                onSelected: () => ref.read(canvasProvider.notifier).resetZoom(),
              ),
            ],
          ),
        ],
      ),

      // ── Help ──
      PlatformMenu(
        label: 'Help',
        menus: [
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'Keyboard Shortcuts',
                shortcut: const SingleActivator(LogicalKeyboardKey.slash, meta: true),
                onSelected: () => ShortcutsDialog.show(_ctx),
              ),
            ],
          ),
          PlatformMenuItemGroup(
            members: [
              PlatformMenuItem(
                label: 'PIXL Website',
                onSelected: () => Process.run('open', ['https://pixl-site.vercel.app']),
              ),
              PlatformMenuItem(
                label: 'Documentation',
                onSelected: () => Process.run('open', ['https://pixl-site.vercel.app/docs']),
              ),
            ],
          ),
        ],
      ),
    ];
  }

  // ── Menu action helpers ──

  Future<void> _save() async {
    final source = await ref.read(backendProvider.notifier).getPaxSource();
    if (source == null) return;
    final ok = await ExportService.quickSavePax(source);
    if (!ok) await ExportService.savePaxSource(source);
    _refreshRecentFiles();
  }

  Future<void> _saveAs() async {
    final source = await ref.read(backendProvider.notifier).getPaxSource();
    final ctx = _ctx;
    if (source != null) {
      final ok = await ExportService.savePaxSource(source);
      if (ctx.mounted) {
        ScaffoldMessenger.of(ctx).showSnackBar(SnackBar(
          content: Text(ok ? 'PAX source saved' : 'Save cancelled'),
          duration: const Duration(seconds: 2),
        ));
      }
    } else if (ctx.mounted) {
      ScaffoldMessenger.of(ctx).showSnackBar(const SnackBar(
        content: Text('No PAX source available (engine not connected?)'),
      ));
    }
  }

  Future<void> _toggleBlueprint() async {
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


  Future<void> _openRecentFile(String path) async {
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
      _syncPalette(resp);
    } else {
      final resp = await ref.read(backendProvider.notifier).loadSource(source);
      _syncPalette(resp);
    }
    await ExportService.setLastFilePath(path);
    _refreshRecentFiles();
  }

  void _syncPalette(Map<String, dynamic> resp) {
    final palette = PixlPalette.fromEngineResponse(resp);
    if (palette != null) {
      ref.read(paletteProvider.notifier).setPalette(palette);
    }
  }
}
