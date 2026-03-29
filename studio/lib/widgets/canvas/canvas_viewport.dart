import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/palette.dart';
import '../../models/pixel_canvas.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/backend_provider.dart';
import '../../providers/hover_provider.dart';
import '../../services/export_service.dart';
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
  double _centerX = 0;
  double _centerY = 0;
  bool _spaceHeld = false;
  // Line/rect tool drag state
  (int, int)? _shapeStart;
  (int, int)? _shapeEnd;
  bool _shiftHeld = false;
  // Selection drag
  (int, int)? _selectStart;
  // Pinch-to-zoom accumulator
  double _pinchAccum = 0.0;

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

  void _handlePointerDown(
    PointerDownEvent event,
    CanvasState cs,
    PixlPalette palette,
  ) {
    final pixel = _pixelFromLocal(event.localPosition, cs);
    if (pixel == null) return;
    final (x, y) = pixel;
    final notifier = ref.read(canvasProvider.notifier);

    switch (cs.activeTool) {
      case DrawingTool.pencil:
        notifier.beginStroke(x, y, palette[cs.foregroundColorIndex]);
        break;
      case DrawingTool.eraser:
        notifier.beginStroke(x, y, null);
        break;
      case DrawingTool.bucket:
        notifier.bucketFill(x, y, palette[cs.foregroundColorIndex]);
        break;
      case DrawingTool.eyedropper:
        final picked = notifier.pickColor(x, y);
        if (picked != null) {
          for (var i = 0; i < palette.length; i++) {
            if (palette[i].toARGB32() == picked.toARGB32()) {
              notifier.setForegroundColor(i);
              break;
            }
          }
        }
        break;
      case DrawingTool.line:
      case DrawingTool.rect:
        setState(() {
          _shapeStart = (x, y);
          _shapeEnd = (x, y);
        });
        break;
      case DrawingTool.rectSelect:
        setState(() => _selectStart = (x, y));
        break;
      case DrawingTool.move:
        break;
    }
  }

  void _handlePointerMove(
    PointerMoveEvent event,
    CanvasState cs,
    PixlPalette palette,
  ) {
    final pixel = _pixelFromLocal(event.localPosition, cs);
    final notifier = ref.read(canvasProvider.notifier);

    final newHover = pixel != null
        ? Offset(pixel.$1.toDouble(), pixel.$2.toDouble())
        : null;
    if (_hoverPixel != newHover) {
      setState(() => _hoverPixel = newHover);
    }

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
        case DrawingTool.line:
        case DrawingTool.rect:
          if (_shapeStart != null) {
            setState(() => _shapeEnd = (x, y));
          }
          break;
        case DrawingTool.rectSelect:
          if (_selectStart != null) {
            final (sx, sy) = _selectStart!;
            final minX = sx < x ? sx : x;
            final minY = sy < y ? sy : y;
            final w = (x - sx).abs() + 1;
            final h = (y - sy).abs() + 1;
            ref.read(selectionProvider.notifier).state = SelectionState(
              x: minX,
              y: minY,
              width: w,
              height: h,
            );
          }
          break;
        default:
          break;
      }
    }
  }

  void _handlePointerUp(
    PointerUpEvent event,
    CanvasState cs,
    PixlPalette palette,
  ) {
    final notifier = ref.read(canvasProvider.notifier);

    // Commit line/rect on mouse up
    if (_shapeStart != null && _shapeEnd != null) {
      final (x0, y0) = _shapeStart!;
      final (x1, y1) = _shapeEnd!;
      final color = cs.activeTool == DrawingTool.eraser
          ? null
          : palette[cs.foregroundColorIndex];

      if (cs.activeTool == DrawingTool.line) {
        notifier.drawLine(x0, y0, x1, y1, color);
      } else if (cs.activeTool == DrawingTool.rect) {
        notifier.drawRect(x0, y0, x1, y1, color, filled: _shiftHeld);
      }
      setState(() {
        _shapeStart = null;
        _shapeEnd = null;
      });
    }

    if (_selectStart != null) {
      setState(() => _selectStart = null);
    }

    notifier.endStroke();
  }

  void _handlePointerHover(PointerHoverEvent event, CanvasState cs) {
    final pixel = _pixelFromLocal(event.localPosition, cs);
    final newHover = pixel != null
        ? Offset(pixel.$1.toDouble(), pixel.$2.toDouble())
        : null;
    if (_hoverPixel == newHover) return;
    setState(() => _hoverPixel = newHover);
    // Update hover provider for status bar display
    final hover = ref.read(hoverProvider.notifier);
    if (pixel != null) {
      hover.update(pixel.$1, pixel.$2);
    } else {
      hover.clear();
    }
  }

  void _handleScroll(PointerSignalEvent event, CanvasState cs) {
    if (event is PointerScaleEvent) {
      // Pinch-to-zoom on trackpad
      _pinchAccum += (event.scale - 1.0);
      if (_pinchAccum > 0.1) {
        ref.read(canvasProvider.notifier).zoomIn();
        _pinchAccum = 0.0;
      } else if (_pinchAccum < -0.1) {
        ref.read(canvasProvider.notifier).zoomOut();
        _pinchAccum = 0.0;
      }
      return;
    }
    if (event is PointerScrollEvent) {
      if (HardwareKeyboard.instance.isMetaPressed) {
        // Cmd + scroll → zoom
        final notifier = ref.read(canvasProvider.notifier);
        if (event.scrollDelta.dy > 0) {
          notifier.zoomOut();
        } else {
          notifier.zoomIn();
        }
      } else {
        // Scroll → pan
        setState(() {
          _panOffset -= event.scrollDelta;
        });
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
            // Track shift for filled rect
            if (event.logicalKey == LogicalKeyboardKey.shiftLeft ||
                event.logicalKey == LogicalKeyboardKey.shiftRight) {
              _shiftHeld = event is KeyDownEvent || event is KeyRepeatEvent;
              return KeyEventResult
                  .ignored; // let other handlers also see shift
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
            // Cmd+S = quick save
            if (meta && event.logicalKey == LogicalKeyboardKey.keyS) {
              _quickSave();
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
            if (event.logicalKey == LogicalKeyboardKey.keyL) {
              notifier.setTool(DrawingTool.line);
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.keyR) {
              notifier.setTool(DrawingTool.rect);
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.keyS && !meta) {
              notifier.setTool(DrawingTool.rectSelect);
              return KeyEventResult.handled;
            }
            // Cmd+C/V/X for clipboard
            if (meta && event.logicalKey == LogicalKeyboardKey.keyC) {
              final sel = ref.read(selectionProvider);
              if (sel.hasSelection) {
                final data = notifier.copyRegion(
                  sel.x,
                  sel.y,
                  sel.width,
                  sel.height,
                );
                ref.read(selectionProvider.notifier).state = sel.copyWith(
                  clipboard: data,
                  clipboardWidth: sel.width,
                  clipboardHeight: sel.height,
                );
              }
              return KeyEventResult.handled;
            }
            if (meta && event.logicalKey == LogicalKeyboardKey.keyV) {
              final sel = ref.read(selectionProvider);
              if (sel.hasClipboard) {
                notifier.pasteRegion(
                  sel.x,
                  sel.y,
                  sel.clipboard!,
                  sel.clipboardWidth,
                  sel.clipboardHeight,
                );
              }
              return KeyEventResult.handled;
            }
            if (meta && event.logicalKey == LogicalKeyboardKey.keyX) {
              final sel = ref.read(selectionProvider);
              if (sel.hasSelection) {
                final data = notifier.copyRegion(
                  sel.x,
                  sel.y,
                  sel.width,
                  sel.height,
                );
                ref.read(selectionProvider.notifier).state = sel.copyWith(
                  clipboard: data,
                  clipboardWidth: sel.width,
                  clipboardHeight: sel.height,
                );
                notifier.clearRegion(sel.x, sel.y, sel.width, sel.height);
              }
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.delete ||
                event.logicalKey == LogicalKeyboardKey.backspace) {
              final sel = ref.read(selectionProvider);
              if (sel.hasSelection) {
                notifier.clearRegion(sel.x, sel.y, sel.width, sel.height);
              }
              return KeyEventResult.handled;
            }
            if (event.logicalKey == LogicalKeyboardKey.escape) {
              ref.read(selectionProvider.notifier).state =
                  const SelectionState();
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
            // Cmd+0 → reset zoom and pan
            if (meta && event.logicalKey == LogicalKeyboardKey.digit0) {
              notifier.resetZoom();
              setState(() => _panOffset = Offset.zero);
              return KeyEventResult.handled;
            }
            return KeyEventResult.ignored;
          },
          child: Listener(
            onPointerSignal: (event) => _handleScroll(event, cs),
            onPointerPanZoomUpdate: (event) {
              // Trackpad two-finger pan
              setState(() {
                _panOffset += event.panDelta;
              });
              // Trackpad pinch-to-zoom
              if (event.scale != 1.0) {
                _pinchAccum += (event.scale - 1.0);
                if (_pinchAccum > 0.1) {
                  ref.read(canvasProvider.notifier).zoomIn();
                  _pinchAccum = 0.0;
                } else if (_pinchAccum < -0.1) {
                  ref.read(canvasProvider.notifier).zoomOut();
                  _pinchAccum = 0.0;
                }
              }
            },
            child: MouseRegion(
              cursor: _spaceHeld
                  ? SystemMouseCursors.grab
                  : _cursorForTool(cs.activeTool),
              onHover: (event) => _handlePointerHover(event, cs),
              onExit: (_) {
                setState(() => _hoverPixel = null);
                ref.read(hoverProvider.notifier).clear();
              },
              child: Listener(
                onPointerDown: _isPanMode(cs)
                    ? null
                    : (event) => _handlePointerDown(event, cs, palette),
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
                onPointerUp: (event) => _handlePointerUp(event, cs, palette),
                child: Container(
                  color: StudioTheme.canvasBg,
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
                            blueprintLandmarks: ref.watch(blueprintProvider),
                            shapePreview:
                                _shapeStart != null && _shapeEnd != null
                                ? (
                                    cs.activeTool,
                                    _shapeStart!,
                                    _shapeEnd!,
                                    _shiftHeld,
                                  )
                                : null,
                            selection: ref.watch(selectionProvider),
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

  Future<void> _quickSave() async {
    final source = await ref.read(backendProvider.notifier).getPaxSource();
    if (source == null) return;
    final ok = await ExportService.quickSavePax(source);
    if (!ok) {
      // No last file path — fall back to save dialog
      await ExportService.savePaxSource(source);
    }
  }

  bool _isPanMode(CanvasState cs) =>
      _spaceHeld || cs.activeTool == DrawingTool.move;

  MouseCursor _cursorForTool(DrawingTool tool) {
    return switch (tool) {
      DrawingTool.pencil => SystemMouseCursors.precise,
      DrawingTool.eraser => SystemMouseCursors.precise,
      DrawingTool.bucket => SystemMouseCursors.click,
      DrawingTool.eyedropper => SystemMouseCursors.click,
      DrawingTool.line => SystemMouseCursors.precise,
      DrawingTool.rect => SystemMouseCursors.precise,
      DrawingTool.rectSelect => SystemMouseCursors.precise,
      DrawingTool.move => SystemMouseCursors.move,
    };
  }
}
