import 'dart:async';
import 'dart:convert';
import 'dart:io' as io;

import 'package:file_picker/file_picker.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/backdrop_provider.dart';
import '../../providers/backend_provider.dart';
import '../../theme/studio_theme.dart';

/// Viewport for viewing and editing backdrops with zone overlays.
class BackdropViewport extends ConsumerStatefulWidget {
  const BackdropViewport({super.key});

  @override
  ConsumerState<BackdropViewport> createState() => _BackdropViewportState();
}

class _BackdropViewportState extends ConsumerState<BackdropViewport> {
  Offset _panOffset = Offset.zero;
  double _zoom = 2.0;
  bool _spaceHeld = false;
  Timer? _animTimer;

  double _pinchAccum = 0.0;

  // Zone dragging state
  int? _draggingZone;
  Offset? _dragStart;

  @override
  void dispose() {
    _animTimer?.cancel();
    super.dispose();
  }

  void _startAnimation() {
    _animTimer?.cancel();
    ref.read(backdropEditorProvider.notifier).setPlaying(true);
    _animTimer = Timer.periodic(const Duration(milliseconds: 120), (_) {
      final state = ref.read(backdropEditorProvider);
      if (!state.isPlaying) {
        _animTimer?.cancel();
        _animTimer = null;
        return;
      }
      final notifier = ref.read(backdropEditorProvider.notifier);
      notifier.setTick(
        (state.currentTick + 1) % state.totalFrames.clamp(1, 999),
      );
      _refreshAnimatedFrame();
    });
  }

  void _stopAnimation() {
    _animTimer?.cancel();
    _animTimer = null;
    ref.read(backdropEditorProvider.notifier).setPlaying(false);
  }

  Future<void> _refreshAnimatedFrame() async {
    final state = ref.read(backdropEditorProvider);
    if (state.paxPath == null || state.backdropName == null) return;
    try {
      final backend = ref.read(backendProvider.notifier).backend;
      final resp = await backend.backdropRender(
        filePath: state.paxPath!,
        name: state.backdropName!,
        frames: 1,
        scale: 1,
      );
      if (resp['ok'] == true && resp['gif_base64'] is String) {
        ref
            .read(backdropEditorProvider.notifier)
            .setAnimatedPreview(base64Decode(resp['gif_base64'] as String));
      }
    } catch (_) {}
  }

  Future<void> _loadBackdrop() async {
    final result = await FilePicker.platform.pickFiles(
      dialogTitle: 'Open Backdrop PAX',
      type: FileType.any,
    );
    if (result == null || result.files.isEmpty) return;
    final path = result.files.single.path;
    if (path == null) return;

    final notifier = ref.read(backdropEditorProvider.notifier);
    notifier.setLoading(true);

    try {
      final backend = ref.read(backendProvider.notifier).backend;

      // Render static preview
      final resp = await backend.backdropRender(
        filePath: path,
        name: 'scene', // Try default name
        scale: 1,
      );

      if (resp['ok'] == true) {
        notifier.loadFromResponse(resp, path, 'scene');
      } else {
        notifier.setError(resp['error']?.toString());
      }
    } catch (e) {
      notifier.setError('Load failed: $e');
    } finally {
      notifier.setLoading(false);
    }
  }

  Offset _toCanvasCoords(Offset local, Size size) {
    final cx = size.width / 2 + _panOffset.dx;
    final cy = size.height / 2 + _panOffset.dy;
    return Offset((local.dx - cx) / _zoom, (local.dy - cy) / _zoom);
  }

  int? _hitTestZone(Offset canvasPos, BackdropEditorState state) {
    for (var i = state.zones.length - 1; i >= 0; i--) {
      final z = state.zones[i];
      if (canvasPos.dx >= z.x &&
          canvasPos.dx < z.x + z.w &&
          canvasPos.dy >= z.y &&
          canvasPos.dy < z.y + z.h) {
        return i;
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(backdropEditorProvider);
    final theme = Theme.of(context);

    if (state.staticPreview == null && !state.loading) {
      // Empty state — show load prompt
      return Container(
        color: StudioTheme.canvasBg,
        child: Center(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.landscape, size: 48, color: theme.dividerColor),
              const SizedBox(height: 12),
              Text(
                'No backdrop loaded',
                style: theme.textTheme.bodySmall!.copyWith(
                  color: theme.dividerColor,
                ),
              ),
              const SizedBox(height: 12),
              ElevatedButton.icon(
                onPressed: _loadBackdrop,
                icon: const Icon(Icons.folder_open, size: 14),
                label: const Text('Open Backdrop PAX'),
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

    if (state.loading) {
      return Container(
        color: StudioTheme.canvasBg,
        child: const Center(child: CircularProgressIndicator(strokeWidth: 2)),
      );
    }

    return Focus(
      autofocus: true,
      onKeyEvent: (node, event) {
        // Track space for pan mode
        if (event.logicalKey == LogicalKeyboardKey.space) {
          final isDown = event is KeyDownEvent || event is KeyRepeatEvent;
          if (_spaceHeld != isDown) setState(() => _spaceHeld = isDown);
          return KeyEventResult.handled;
        }
        if (event is! KeyDownEvent) return KeyEventResult.ignored;
        final meta = HardwareKeyboard.instance.isMetaPressed;
        // +/= and - for zoom
        if (event.logicalKey == LogicalKeyboardKey.equal ||
            event.logicalKey == LogicalKeyboardKey.numpadAdd) {
          setState(() => _zoom = (_zoom * 1.2).clamp(0.5, 16.0));
          return KeyEventResult.handled;
        }
        if (event.logicalKey == LogicalKeyboardKey.minus ||
            event.logicalKey == LogicalKeyboardKey.numpadSubtract) {
          setState(() => _zoom = (_zoom / 1.2).clamp(0.5, 16.0));
          return KeyEventResult.handled;
        }
        // Cmd+0 → reset zoom and pan
        if (meta && event.logicalKey == LogicalKeyboardKey.digit0) {
          setState(() {
            _zoom = 2.0;
            _panOffset = Offset.zero;
          });
          return KeyEventResult.handled;
        }
        return KeyEventResult.ignored;
      },
      child: LayoutBuilder(
        builder: (context, constraints) {
          return Listener(
            onPointerSignal: (signal) {
              if (signal is PointerScaleEvent) {
                // Pinch-to-zoom on trackpad
                _pinchAccum += (signal.scale - 1.0);
                if (_pinchAccum.abs() > 0.1) {
                  setState(() {
                    if (_pinchAccum > 0) {
                      _zoom = (_zoom * 1.2).clamp(0.5, 16.0);
                    } else {
                      _zoom = (_zoom / 1.2).clamp(0.5, 16.0);
                    }
                  });
                  _pinchAccum = 0.0;
                }
                return;
              }
              if (signal is PointerScrollEvent) {
                if (HardwareKeyboard.instance.isMetaPressed) {
                  // Cmd + scroll → zoom
                  setState(() {
                    if (signal.scrollDelta.dy > 0) {
                      _zoom = (_zoom / 1.2).clamp(0.5, 16.0);
                    } else {
                      _zoom = (_zoom * 1.2).clamp(0.5, 16.0);
                    }
                  });
                } else {
                  // Scroll → pan
                  setState(() {
                    _panOffset -= signal.scrollDelta;
                  });
                }
              }
            },
            onPointerPanZoomUpdate: (event) {
              setState(() {
                _panOffset += event.panDelta;
              });
              if (event.scale != 1.0) {
                _pinchAccum += (event.scale - 1.0);
                if (_pinchAccum.abs() > 0.1) {
                  setState(() {
                    if (_pinchAccum > 0) {
                      _zoom = (_zoom * 1.2).clamp(0.5, 16.0);
                    } else {
                      _zoom = (_zoom / 1.2).clamp(0.5, 16.0);
                    }
                  });
                  _pinchAccum = 0.0;
                }
              }
            },
            child: MouseRegion(
              cursor: _spaceHeld
                  ? SystemMouseCursors.grab
                  : SystemMouseCursors.basic,
              child: GestureDetector(
                onPanStart: (details) {
                  if (_spaceHeld) return; // pan mode handled by onPanUpdate
                  final canvas = _toCanvasCoords(
                    details.localPosition,
                    constraints.biggest,
                  );
                  final hit = _hitTestZone(canvas, state);
                  if (hit != null) {
                    ref.read(backdropEditorProvider.notifier).selectZone(hit);
                    _draggingZone = hit;
                    _dragStart = canvas;
                  }
                },
                onPanUpdate: (details) {
                  if (_spaceHeld) {
                    setState(() => _panOffset += details.delta);
                    return;
                  }
                  if (_draggingZone != null && _dragStart != null) {
                    final canvas = _toCanvasCoords(
                      details.localPosition,
                      constraints.biggest,
                    );
                    final dx = (canvas.dx - _dragStart!.dx).round();
                    final dy = (canvas.dy - _dragStart!.dy).round();
                    final zone = state.zones[_draggingZone!];
                    ref
                        .read(backdropEditorProvider.notifier)
                        .updateZone(
                          _draggingZone!,
                          zone.copyWith(x: zone.x + dx, y: zone.y + dy),
                        );
                    _dragStart = canvas;
                  }
                },
                onPanEnd: (_) {
                  _draggingZone = null;
                  _dragStart = null;
                },
                child: Container(
                  color: StudioTheme.canvasBg,
                  child: Stack(
                    children: [
                      // Centered image + zone overlay
                      Center(
                        child: Transform.translate(
                          offset: _panOffset,
                          child: Transform.scale(
                            scale: _zoom,
                            filterQuality: FilterQuality.none,
                            child: Stack(
                              children: [
                                // Base image
                                Image.memory(
                                  state.isPlaying &&
                                          state.animatedPreview != null
                                      ? state.animatedPreview!
                                      : state.staticPreview!,
                                  filterQuality: FilterQuality.none,
                                  fit: BoxFit.none,
                                  gaplessPlayback: true,
                                ),
                                // Zone overlays
                                if (state.zones.isNotEmpty)
                                  Positioned.fill(
                                    child: CustomPaint(
                                      painter: _BackdropPainter(
                                        zones: state.zones,
                                        selectedZone: state.selectedZoneIndex,
                                      ),
                                    ),
                                  ),
                              ],
                            ),
                          ),
                        ),
                      ),

                      // Animation controls bar at bottom
                      Positioned(
                        left: 0,
                        right: 0,
                        bottom: 0,
                        child: _AnimationBar(
                          isPlaying: state.isPlaying,
                          tick: state.currentTick,
                          totalFrames: state.totalFrames,
                          onPlay: _startAnimation,
                          onPause: _stopAnimation,
                          onSeek: (t) => ref
                              .read(backdropEditorProvider.notifier)
                              .setTick(t),
                          onExport: () async {
                            if (state.paxPath == null ||
                                state.backdropName == null)
                              return;
                            final backend = ref
                                .read(backendProvider.notifier)
                                .backend;
                            final resp = await backend.backdropRender(
                              filePath: state.paxPath!,
                              name: state.backdropName!,
                              frames: state.totalFrames,
                              scale: 4,
                            );
                            if (resp['ok'] == true &&
                                resp['gif_base64'] is String) {
                              final dir = await FilePicker.platform
                                  .getDirectoryPath(
                                    dialogTitle: 'Save Animated GIF',
                                  );
                              if (dir != null) {
                                final bytes = base64Decode(
                                  resp['gif_base64'] as String,
                                );
                                final path =
                                    '$dir/${state.backdropName}_animated.gif';
                                await io.File(path).writeAsBytes(bytes);
                              }
                            }
                          },
                        ),
                      ),

                      // Zoom indicator
                      Positioned(
                        right: 8,
                        top: 8,
                        child: Container(
                          padding: const EdgeInsets.symmetric(
                            horizontal: 8,
                            vertical: 4,
                          ),
                          decoration: BoxDecoration(
                            color: Colors.black54,
                            borderRadius: BorderRadius.circular(4),
                          ),
                          child: Text(
                            '${_zoom.toStringAsFixed(1)}x',
                            style: const TextStyle(
                              color: Colors.white,
                              fontSize: 10,
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          );
        },
      ),
    );
  }
}

/// Paints the backdrop image with zone overlays.
class _BackdropPainter extends CustomPainter {
  final List<ZoneState> zones;
  final int? selectedZone;

  _BackdropPainter({required this.zones, this.selectedZone});

  static const _zoneColors = [
    Color(0x404CAF50), // green
    Color(0x402196F3), // blue
    Color(0x40FF9800), // orange
    Color(0x409C27B0), // purple
    Color(0x40F44336), // red
    Color(0x4000BCD4), // cyan
    Color(0x40FFEB3B), // yellow
    Color(0x40795548), // brown
  ];

  @override
  void paint(Canvas canvas, Size size) {
    // The image is drawn by the parent Image.memory widget
    // We only draw zone overlays here

    for (var i = 0; i < zones.length; i++) {
      final z = zones[i];
      final rect = Rect.fromLTWH(
        z.x.toDouble(),
        z.y.toDouble(),
        z.w.toDouble(),
        z.h.toDouble(),
      );
      final color = _zoneColors[i % _zoneColors.length];

      // Fill
      canvas.drawRect(rect, Paint()..color = color);

      // Border
      final borderColor = i == selectedZone
          ? Colors.white
          : color.withValues(alpha: 0.8);
      canvas.drawRect(
        rect,
        Paint()
          ..color = borderColor
          ..style = PaintingStyle.stroke
          ..strokeWidth = i == selectedZone ? 2.0 : 1.0,
      );

      // Label
      final tp = TextPainter(
        text: TextSpan(
          text: '${z.name} (${z.behavior})',
          style: TextStyle(
            color: Colors.white,
            fontSize: 8,
            backgroundColor: Colors.black54,
          ),
        ),
        textDirection: TextDirection.ltr,
      )..layout(maxWidth: z.w.toDouble());
      tp.paint(canvas, Offset(z.x.toDouble() + 2, z.y.toDouble() + 2));
    }
  }

  @override
  bool shouldRepaint(covariant _BackdropPainter oldDelegate) {
    return oldDelegate.zones != zones ||
        oldDelegate.selectedZone != selectedZone;
  }
}

/// Animation playback controls bar.
class _AnimationBar extends StatelessWidget {
  const _AnimationBar({
    required this.isPlaying,
    required this.tick,
    required this.totalFrames,
    required this.onPlay,
    required this.onPause,
    required this.onSeek,
    required this.onExport,
  });

  final bool isPlaying;
  final int tick;
  final int totalFrames;
  final VoidCallback onPlay;
  final VoidCallback onPause;
  final ValueChanged<int> onSeek;
  final VoidCallback onExport;

  @override
  Widget build(BuildContext context) {
    return Container(
      height: 36,
      decoration: const BoxDecoration(
        color: Color(0xDD1E1E2E),
        border: Border(top: BorderSide(color: Color(0x33FFFFFF))),
      ),
      padding: const EdgeInsets.symmetric(horizontal: 12),
      child: Row(
        children: [
          // Play/Pause
          IconButton(
            icon: Icon(isPlaying ? Icons.pause : Icons.play_arrow, size: 18),
            color: Colors.white,
            onPressed: isPlaying ? onPause : onPlay,
            visualDensity: VisualDensity.compact,
            tooltip: isPlaying ? 'Pause' : 'Play',
          ),
          // Frame slider
          Expanded(
            child: Slider(
              value: tick.toDouble(),
              min: 0,
              max: (totalFrames - 1).toDouble().clamp(0, 999),
              divisions: totalFrames > 1 ? totalFrames - 1 : null,
              onChanged: (v) => onSeek(v.round()),
            ),
          ),
          // Frame counter
          Text(
            '${tick + 1}/$totalFrames',
            style: const TextStyle(color: Colors.white70, fontSize: 10),
          ),
          const SizedBox(width: 8),
          // Export button
          IconButton(
            icon: const Icon(Icons.file_download, size: 16),
            color: Colors.white70,
            onPressed: onExport,
            visualDensity: VisualDensity.compact,
            tooltip: 'Export GIF',
          ),
        ],
      ),
    );
  }
}
