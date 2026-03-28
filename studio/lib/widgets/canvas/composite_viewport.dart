import 'dart:convert';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/backend_provider.dart';
import '../../providers/composite_provider.dart';
import '../../theme/studio_theme.dart';

/// Viewport for viewing and editing composite sprites.
class CompositeViewport extends ConsumerStatefulWidget {
  const CompositeViewport({super.key});

  @override
  ConsumerState<CompositeViewport> createState() => _CompositeViewportState();
}

class _CompositeViewportState extends ConsumerState<CompositeViewport> {
  Offset _panOffset = Offset.zero;
  double _zoom = 4.0;
  bool _spaceHeld = false;
  double _pinchAccum = 0.0;

  @override
  void initState() {
    super.initState();
    _loadComposites();
  }

  Future<void> _loadComposites() async {
    final notifier = ref.read(compositeEditorProvider.notifier);
    notifier.setLoading(true);
    try {
      final backend = ref.read(backendProvider.notifier).backend;
      final resp = await backend.callTool('pixl_list_composites', {});
      if (resp.containsKey('error')) {
        notifier.setError(resp['error']?.toString());
        return;
      }
      final list = (resp['composites'] as List? ?? []);
      final composites = list
          .map((c) => CompositeInfo(
                name: c['name'] ?? '',
                size: c['size'] ?? '',
                tileSize: c['tile_size'] ?? '',
                variants:
                    (c['variants'] as List?)?.cast<String>() ?? [],
                animations:
                    (c['animations'] as List?)?.cast<String>() ?? [],
              ))
          .toList();
      notifier.setComposites(composites);
    } catch (e) {
      notifier.setError('Load failed: $e');
    } finally {
      notifier.setLoading(false);
    }
  }

  Future<void> _renderPreview() async {
    final state = ref.read(compositeEditorProvider);
    if (state.selectedName == null) return;

    final notifier = ref.read(compositeEditorProvider.notifier);
    try {
      final backend = ref.read(backendProvider.notifier).backend;
      final args = <String, dynamic>{
        'name': state.selectedName,
        'scale': 8,
      };
      if (state.selectedVariant != null) args['variant'] = state.selectedVariant;
      if (state.selectedAnim != null) args['anim'] = state.selectedAnim;
      if (state.selectedFrame != null) args['frame'] = state.selectedFrame;

      final resp = await backend.callTool('pixl_render_composite', args);
      if (resp['preview_b64'] is String) {
        notifier.setPreview(base64Decode(resp['preview_b64'] as String));
      } else if (resp.containsKey('error')) {
        notifier.setError(resp['error']?.toString());
      }
    } catch (e) {
      notifier.setError('Render failed: $e');
    }
  }

  Future<void> _checkSeams() async {
    final notifier = ref.read(compositeEditorProvider.notifier);
    try {
      final backend = ref.read(backendProvider.notifier).backend;
      final resp = await backend.callTool('pixl_check_seams', {});
      if (resp['warnings'] is List) {
        notifier.setSeamWarnings(
          (resp['warnings'] as List).cast<Map<String, dynamic>>(),
        );
      }
    } catch (_) {}
  }

  void _selectComposite(CompositeInfo info) {
    ref.read(compositeEditorProvider.notifier).select(info.name);
    _renderPreview();
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(compositeEditorProvider);
    final theme = Theme.of(context);

    if (state.loading) {
      return Container(
        color: StudioTheme.canvasBg,
        child: const Center(child: CircularProgressIndicator(strokeWidth: 2)),
      );
    }

    if (state.composites.isEmpty) {
      return Container(
        color: StudioTheme.canvasBg,
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.dashboard, size: 48, color: theme.dividerColor),
              const SizedBox(height: 12),
              Text(
                'No composites defined',
                style: theme.textTheme.bodySmall!.copyWith(
                  color: theme.dividerColor,
                ),
              ),
              const SizedBox(height: 4),
              Text(
                'Add [composite.<name>] sections to your .pax file',
                style: theme.textTheme.bodySmall!.copyWith(
                  color: theme.dividerColor.withValues(alpha: 0.6),
                  fontSize: 11,
                ),
              ),
              const SizedBox(height: 12),
              ElevatedButton.icon(
                onPressed: _loadComposites,
                icon: const Icon(Icons.refresh, size: 14),
                label: const Text('Refresh'),
                style: ElevatedButton.styleFrom(
                  backgroundColor: theme.colorScheme.primary,
                  foregroundColor: Colors.white,
                  textStyle: const TextStyle(fontSize: 12),
                ),
              ),
            ],
          ),
        ),
      );
    }

    return Focus(
      autofocus: true,
      onKeyEvent: (node, event) {
        if (event.logicalKey == LogicalKeyboardKey.space) {
          final isDown = event is KeyDownEvent || event is KeyRepeatEvent;
          if (_spaceHeld != isDown) setState(() => _spaceHeld = isDown);
          return KeyEventResult.handled;
        }
        if (event is! KeyDownEvent) return KeyEventResult.ignored;
        if (event.logicalKey == LogicalKeyboardKey.equal ||
            event.logicalKey == LogicalKeyboardKey.numpadAdd) {
          setState(() => _zoom = (_zoom * 1.2).clamp(1.0, 32.0));
          return KeyEventResult.handled;
        }
        if (event.logicalKey == LogicalKeyboardKey.minus ||
            event.logicalKey == LogicalKeyboardKey.numpadSubtract) {
          setState(() => _zoom = (_zoom / 1.2).clamp(1.0, 32.0));
          return KeyEventResult.handled;
        }
        return KeyEventResult.ignored;
      },
      child: Row(
        children: [
          // Left panel: composite list + controls
          SizedBox(
            width: 200,
            child: Container(
              color: const Color(0xFF1E1E2E),
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  // Header
                  Container(
                    padding: const EdgeInsets.all(8),
                    decoration: const BoxDecoration(
                      border: Border(bottom: BorderSide(color: Color(0x33FFFFFF))),
                    ),
                    child: Row(
                      children: [
                        const Text(
                          'Composites',
                          style: TextStyle(color: Colors.white, fontSize: 12, fontWeight: FontWeight.bold),
                        ),
                        const Spacer(),
                        IconButton(
                          icon: const Icon(Icons.refresh, size: 14),
                          color: Colors.white54,
                          onPressed: _loadComposites,
                          visualDensity: VisualDensity.compact,
                          tooltip: 'Refresh',
                        ),
                        IconButton(
                          icon: const Icon(Icons.flaky, size: 14),
                          color: Colors.white54,
                          onPressed: _checkSeams,
                          visualDensity: VisualDensity.compact,
                          tooltip: 'Check Seams',
                        ),
                      ],
                    ),
                  ),
                  // Composite list
                  Expanded(
                    child: ListView.builder(
                      itemCount: state.composites.length,
                      itemBuilder: (context, i) {
                        final c = state.composites[i];
                        final selected = c.name == state.selectedName;
                        return ListTile(
                          dense: true,
                          selected: selected,
                          selectedTileColor: theme.colorScheme.primary.withValues(alpha: 0.15),
                          title: Text(
                            c.name,
                            style: TextStyle(
                              color: selected ? Colors.white : Colors.white70,
                              fontSize: 12,
                            ),
                          ),
                          subtitle: Text(
                            '${c.size} (${c.variants.length}v ${c.animations.length}a)',
                            style: const TextStyle(color: Colors.white38, fontSize: 10),
                          ),
                          onTap: () => _selectComposite(c),
                        );
                      },
                    ),
                  ),
                  // Variant/animation selectors
                  if (state.selected != null) ...[
                    const Divider(color: Color(0x33FFFFFF), height: 1),
                    if (state.selected!.variants.isNotEmpty)
                      Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                        child: DropdownButton<String?>(
                          value: state.selectedVariant,
                          isExpanded: true,
                          hint: const Text('Base', style: TextStyle(color: Colors.white38, fontSize: 11)),
                          dropdownColor: const Color(0xFF2A2A3E),
                          style: const TextStyle(color: Colors.white70, fontSize: 11),
                          underline: const SizedBox(),
                          items: [
                            const DropdownMenuItem(value: null, child: Text('Base layout')),
                            ...state.selected!.variants.map((v) =>
                              DropdownMenuItem(value: v, child: Text(v)),
                            ),
                          ],
                          onChanged: (v) {
                            ref.read(compositeEditorProvider.notifier).selectVariant(v);
                            _renderPreview();
                          },
                        ),
                      ),
                    if (state.selected!.animations.isNotEmpty)
                      Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                        child: DropdownButton<String?>(
                          value: state.selectedAnim,
                          isExpanded: true,
                          hint: const Text('No animation', style: TextStyle(color: Colors.white38, fontSize: 11)),
                          dropdownColor: const Color(0xFF2A2A3E),
                          style: const TextStyle(color: Colors.white70, fontSize: 11),
                          underline: const SizedBox(),
                          items: [
                            const DropdownMenuItem(value: null, child: Text('Static')),
                            ...state.selected!.animations.map((a) =>
                              DropdownMenuItem(value: a, child: Text(a)),
                            ),
                          ],
                          onChanged: (v) {
                            ref.read(compositeEditorProvider.notifier).selectAnim(v);
                            _renderPreview();
                          },
                        ),
                      ),
                    if (state.selectedAnim != null)
                      Padding(
                        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                        child: Row(
                          children: [
                            const Text('Frame:', style: TextStyle(color: Colors.white38, fontSize: 10)),
                            const SizedBox(width: 4),
                            ...List.generate(4, (i) {
                              final f = i + 1;
                              final active = state.selectedFrame == f;
                              return Padding(
                                padding: const EdgeInsets.only(right: 4),
                                child: GestureDetector(
                                  onTap: () {
                                    ref.read(compositeEditorProvider.notifier).selectFrame(f);
                                    _renderPreview();
                                  },
                                  child: Container(
                                    width: 24,
                                    height: 24,
                                    alignment: Alignment.center,
                                    decoration: BoxDecoration(
                                      color: active ? theme.colorScheme.primary : const Color(0xFF2A2A3E),
                                      borderRadius: BorderRadius.circular(4),
                                    ),
                                    child: Text(
                                      '$f',
                                      style: TextStyle(
                                        color: active ? Colors.white : Colors.white54,
                                        fontSize: 10,
                                      ),
                                    ),
                                  ),
                                ),
                              );
                            }),
                          ],
                        ),
                      ),
                  ],
                  // Seam warnings
                  if (state.seamWarnings.isNotEmpty) ...[
                    const Divider(color: Color(0x33FFFFFF), height: 1),
                    Container(
                      padding: const EdgeInsets.all(8),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          Text(
                            '${state.seamWarnings.length} seam warning(s)',
                            style: const TextStyle(color: Color(0xFFFF9800), fontSize: 10),
                          ),
                          ...state.seamWarnings.take(5).map((w) => Text(
                            '${w["composite"]}: ${w["slot_a"]}-${w["slot_b"]} (${w["direction"]})',
                            style: const TextStyle(color: Colors.white38, fontSize: 9),
                          )),
                        ],
                      ),
                    ),
                  ],
                ],
              ),
            ),
          ),
          // Main preview area
          Expanded(
            child: Listener(
              onPointerSignal: (signal) {
                if (signal is PointerScaleEvent) {
                  _pinchAccum += (signal.scale - 1.0);
                  if (_pinchAccum.abs() > 0.1) {
                    setState(() {
                      _zoom = _pinchAccum > 0
                          ? (_zoom * 1.2).clamp(1.0, 32.0)
                          : (_zoom / 1.2).clamp(1.0, 32.0);
                    });
                    _pinchAccum = 0.0;
                  }
                  return;
                }
                if (signal is PointerScrollEvent) {
                  if (HardwareKeyboard.instance.isMetaPressed) {
                    setState(() {
                      _zoom = signal.scrollDelta.dy > 0
                          ? (_zoom / 1.2).clamp(1.0, 32.0)
                          : (_zoom * 1.2).clamp(1.0, 32.0);
                    });
                  } else {
                    setState(() => _panOffset -= signal.scrollDelta);
                  }
                }
              },
              child: MouseRegion(
                cursor: _spaceHeld ? SystemMouseCursors.grab : SystemMouseCursors.basic,
                child: GestureDetector(
                  onPanUpdate: (details) {
                    if (_spaceHeld) {
                      setState(() => _panOffset += details.delta);
                    }
                  },
                  child: Container(
                    color: StudioTheme.canvasBg,
                    child: Stack(
                      children: [
                        if (state.preview != null)
                          Center(
                            child: Transform.translate(
                              offset: _panOffset,
                              child: Transform.scale(
                                scale: _zoom,
                                filterQuality: FilterQuality.none,
                                child: Image.memory(
                                  state.preview!,
                                  filterQuality: FilterQuality.none,
                                  fit: BoxFit.none,
                                  gaplessPlayback: true,
                                ),
                              ),
                            ),
                          )
                        else
                          Center(
                            child: Text(
                              state.selectedName != null
                                  ? 'Select a composite to preview'
                                  : 'Select a composite from the list',
                              style: TextStyle(
                                color: theme.dividerColor,
                                fontSize: 12,
                              ),
                            ),
                          ),
                        // Zoom indicator
                        Positioned(
                          right: 8,
                          top: 8,
                          child: Container(
                            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                            decoration: BoxDecoration(
                              color: Colors.black54,
                              borderRadius: BorderRadius.circular(4),
                            ),
                            child: Text(
                              '${_zoom.toStringAsFixed(1)}x',
                              style: const TextStyle(color: Colors.white, fontSize: 10),
                            ),
                          ),
                        ),
                        // Error display
                        if (state.error != null)
                          Positioned(
                            left: 8,
                            bottom: 8,
                            child: Container(
                              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
                              decoration: BoxDecoration(
                                color: const Color(0xDDF44336),
                                borderRadius: BorderRadius.circular(4),
                              ),
                              child: Text(
                                state.error!,
                                style: const TextStyle(color: Colors.white, fontSize: 10),
                              ),
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
