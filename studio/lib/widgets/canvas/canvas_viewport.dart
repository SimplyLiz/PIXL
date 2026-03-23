import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/palette.dart';
import '../../models/pixel_canvas.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/hover_provider.dart';
import '../../theme/studio_theme.dart';
import '../../providers/palette_provider.dart';
import '../shortcuts_dialog.dart';
import 'pixel_canvas_painter.dart';

/// The main canvas viewport — handles mouse input, zooming, and delegates
/// to PixelCanvasPainter for rendering.
class CanvasViewport extends ConsumerStatefulWidget {
  const CanvasViewport({super.key});

  @override
  ConsumerState<CanvasViewport> createState() => _CanvasViewportState();
}

class _CanvasViewportState extends ConsumerState<CanvasViewport> {
  Offset? _hoverPixel;
  Offset _panOffset = Offset.zero;
  // Centering offset — updated each build from LayoutBuilder constraints.
  double _centerX = 0;
  double _centerY = 0;
  // Space key held = pan mode (overrides drawing).
  bool _spaceHeld = false;

  (int, int)? _pixelFromLocal(Offset localPos, CanvasState cs) {
    final ps = cs.zoomLevel;

    // localPos is relative to the full viewport container.
    // The canvas is positioned at (centerX + panOffset) inside the Stack,
    // so subtract both to get coordinates relative to the canvas origin.
    final dx = localPos.dx - _centerX - _panOffset.dx;
    final dy = localPos.dy - _centerY - _panOffset.dy;

    final x = (dx / ps).floor();
    final y = (dy / ps).floor();

    if (x < 0 || x >= cs.width || y < 0 || y >= cs.height) return null;
    return (x, y);
  }

  void _handlePointerDown(PointerDownEvent event, CanvasState cs, PixlPalette palette) {
    final pixel = _pixelFromLocal(event.localPosition, cs);
    if (pixel == null) return;
    final (x, y) = pixel;
    final notifier = ref.read(canvasProvider.notifier);

    switch (cs.activeTool) {
      case DrawingTool.pencil:
        final color = palette[cs.foregroundColorIndex];
        notifier.beginStroke(x, y, color);
        break;
      case DrawingTool.eraser:
        notifier.beginStroke(x, y, null);
        break;
      case DrawingTool.bucket:
        final color = palette[cs.foregroundColorIndex];
        notifier.bucketFill(x, y, color);
        break;
      case DrawingTool.eyedropper:
        final picked = notifier.pickColor(x, y);
        if (picked != null) {
          // Find matching palette color
          for (var i = 0; i < palette.length; i++) {
            if (palette[i].toARGB32() == picked.toARGB32()) {
              notifier.setForegroundColor(i);
              break;
            }
          }
        }
        break;
      case DrawingTool.rectSelect:
      case DrawingTool.move:
        // TODO: implement selection/move
        break;
    }
  }

  void _handlePointerMove(PointerMoveEvent event, CanvasState cs, PixlPalette palette) {
    final pixel = _pixelFromLocal(event.localPosition, cs);
    final notifier = ref.read(canvasProvider.notifier);

    setState(() {
      _hoverPixel = pixel != null ? Offset(pixel.$1.toDouble(), pixel.$2.toDouble()) : null;
    });

    if (pixel == null) return;
    final (x, y) = pixel;

    if (event.buttons != 0) {
      switch (cs.activeTool) {
        case DrawingTool.pencil:
          notifier.continueStroke(x, y, palette[cs.foregroundColorIndex]);
          break;
        case DrawingTool.eraser:
          notifier.continueStroke(x, y, null);
          break;
        default:
          break;
      }
    }
  }

  void _handlePointerUp(PointerUpEvent event) {
    ref.read(canvasProvider.notifier).endStroke();
  }

  void _handlePointerHover(PointerHoverEvent event, CanvasState cs) {
    final pixel = _pixelFromLocal(event.localPosition, cs);
    setState(() {
      _hoverPixel = pixel != null ? Offset(pixel.$1.toDouble(), pixel.$2.toDouble()) : null;
    });
    // Update hover provider for status bar display
    final hover = ref.read(hoverProvider.notifier);
    if (pixel != null) {
      hover.update(pixel.$1, pixel.$2);
    } else {
      hover.clear();
    }
  }

  void _handleScroll(PointerSignalEvent event, CanvasState cs) {
    if (event is PointerScrollEvent) {
      final notifier = ref.read(canvasProvider.notifier);
      if (event.scrollDelta.dy > 0) {
        notifier.zoomOut();
      } else {
        notifier.zoomIn();
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final cs = ref.watch(canvasProvider);
    final palette = ref.watch(paletteProvider);
    final ps = cs.zoomLevel;
    final canvasW = cs.width * ps;
    final canvasH = cs.height * ps;

    return LayoutBuilder(
      builder: (context, constraints) {
        // Center the canvas in the viewport — store for hit-testing
        _centerX = (constraints.maxWidth - canvasW) / 2;
        _centerY = (constraints.maxHeight - canvasH) / 2;

        return Focus(
          autofocus: true,
          onKeyEvent: (node, event) {
            // Track space for pan mode
            if (event.logicalKey == LogicalKeyboardKey.space) {
              final isDown = event is KeyDownEvent || event is KeyRepeatEvent;
              if (_spaceHeld != isDown) {
                setState(() => _spaceHeld = isDown);
              }
              return KeyEventResult.handled;
            }

            if (event is! KeyDownEvent) return KeyEventResult.ignored;
            final notifier = ref.read(canvasProvider.notifier);
            final meta = HardwareKeyboard.instance.isMetaPressed;
            final shift = HardwareKeyboard.instance.isShiftPressed;

            if (meta && event.logicalKey == LogicalKeyboardKey.keyZ) {
              if (shift) {
                notifier.redo();
              } else {
                notifier.undo();
              }
              return KeyEventResult.handled;
            }
            // Tool shortcuts
            if (event.logicalKey == LogicalKeyboardKey.keyB) {
              notifier.setTool(DrawingTool.pencil);
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.keyE) {
              notifier.setTool(DrawingTool.eraser);
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.keyG) {
              notifier.setTool(DrawingTool.bucket);
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.keyI) {
              notifier.setTool(DrawingTool.eyedropper);
              return KeyEventResult.handled;
            }
            // H = toggle grid
            if (event.logicalKey == LogicalKeyboardKey.keyH && !meta) {
              notifier.toggleGrid();
              return KeyEventResult.handled;
            }
            // Cmd+/ = shortcuts overlay
            if (meta && event.logicalKey == LogicalKeyboardKey.slash) {
              ShortcutsDialog.show(context);
              return KeyEventResult.handled;
            }
            // +/= and - for zoom
            if (event.logicalKey == LogicalKeyboardKey.equal ||
                event.logicalKey == LogicalKeyboardKey.numpadAdd) {
              notifier.zoomIn();
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.minus ||
                event.logicalKey == LogicalKeyboardKey.numpadSubtract) {
              notifier.zoomOut();
              return KeyEventResult.handled;
            }
            return KeyEventResult.ignored;
          },
          child: Listener(
            onPointerSignal: (event) => _handleScroll(event, cs),
            child: MouseRegion(
              cursor: _spaceHeld ? SystemMouseCursors.grab : _cursorForTool(cs.activeTool),
              onHover: (event) => _handlePointerHover(event, cs),
              onExit: (_) {
                setState(() => _hoverPixel = null);
                ref.read(hoverProvider.notifier).clear();
              },
              child: Listener(
                onPointerDown: _isPanMode(cs) ? null : (event) => _handlePointerDown(event, cs, palette),
                onPointerMove: (event) {
                  if (_isPanMode(cs) && event.buttons != 0) {
                    // Pan mode: drag to pan
                    setState(() {
                      _panOffset += event.delta;
                    });
                  } else {
                    _handlePointerMove(event, cs, palette);
                  }
                },
                onPointerUp: (event) => _handlePointerUp(event),
                child: Container(
                  color: const StudioTheme.canvasBg,
                  child: Stack(
                    children: [
                      Positioned(
                        left: _centerX + _panOffset.dx,
                        top: _centerY + _panOffset.dy,
                        child: CustomPaint(
                          size: Size(canvasW, canvasH),
                          painter: PixelCanvasPainter(
                            canvasState: cs,
                            pixelSize: ps,
                            hoverPixel: _hoverPixel,
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }

  bool _isPanMode(CanvasState cs) =>
      _spaceHeld || cs.activeTool == DrawingTool.move;

  MouseCursor _cursorForTool(DrawingTool tool) {
    return switch (tool) {
      DrawingTool.pencil => SystemMouseCursors.precise,
      DrawingTool.eraser => SystemMouseCursors.precise,
      DrawingTool.bucket => SystemMouseCursors.click,
      DrawingTool.eyedropper => SystemMouseCursors.click,
      DrawingTool.rectSelect => SystemMouseCursors.precise,
      DrawingTool.move => SystemMouseCursors.move,
    };
  }
}
