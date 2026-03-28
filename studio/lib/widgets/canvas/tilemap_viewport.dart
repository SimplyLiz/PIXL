import 'dart:convert';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/pixel_canvas.dart';
import '../../providers/backend_provider.dart';
import '../../providers/tilemap_provider.dart';

/// Tilemap canvas — paint tiles on a 2D grid.
class TilemapViewport extends ConsumerStatefulWidget {
  const TilemapViewport({super.key});

  @override
  ConsumerState<TilemapViewport> createState() => _TilemapViewportState();
}

class _TilemapViewportState extends ConsumerState<TilemapViewport> {
  final _tileImages = <String, ui.Image>{};
  final _loadingTiles = <String>{};
  (int, int)? _hoverTile;
  double _centerX = 0;
  double _centerY = 0;
  bool _spaceHeld = false;
  Offset _panOffset = Offset.zero;
  Offset _panStart = Offset.zero;
  double _pinchAccum = 0.0;

  @override
  void initState() {
    super.initState();
    // Load tile images when the tile list changes, not on every build.
    ref.listenManual(
      backendProvider.select((s) => s.tiles),
      (_, tiles) => _ensureTileImages(tiles),
      fireImmediately: true,
    );
  }

  @override
  void dispose() {
    for (final img in _tileImages.values) {
      img.dispose();
    }
    super.dispose();
  }

  // ── Tile image cache ─────────────────────────────

  Future<void> _ensureTileImages(List<TileInfo> tiles) async {
    for (final tile in tiles) {
      if (_tileImages.containsKey(tile.name) ||
          _loadingTiles.contains(tile.name))
        continue;
      final bytes = tile.previewBase64 != null
          ? base64Decode(tile.previewBase64!)
          : null;
      if (bytes == null) {
        // Fetch from backend
        _loadingTiles.add(tile.name);
        final b64 = await ref
            .read(backendProvider.notifier)
            .renderTile(tile.name, scale: 1);
        _loadingTiles.remove(tile.name);
        if (b64 != null && mounted) {
          final decoded = await _decodeImage(base64Decode(b64));
          if (decoded != null && mounted) {
            setState(() => _tileImages[tile.name] = decoded);
          }
        }
      } else {
        _loadingTiles.add(tile.name);
        final decoded = await _decodeImage(bytes);
        _loadingTiles.remove(tile.name);
        if (decoded != null && mounted) {
          setState(() => _tileImages[tile.name] = decoded);
        }
      }
    }
  }

  Future<ui.Image?> _decodeImage(List<int> bytes) async {
    try {
      final codec = await ui.instantiateImageCodec(bytes as Uint8List);
      final frame = await codec.getNextFrame();
      return frame.image;
    } catch (_) {
      return null;
    }
  }

  // ── Hit testing ──────────────────────────────────

  (int, int)? _tileFromLocal(Offset localPos, TilemapState ts) {
    final cellSize = ts.tilePixelSize * ts.zoomLevel;
    final dx = localPos.dx - _centerX - _panOffset.dx;
    final dy = localPos.dy - _centerY - _panOffset.dy;
    final col = (dx / cellSize).floor();
    final row = (dy / cellSize).floor();
    if (col < 0 || col >= ts.gridWidth || row < 0 || row >= ts.gridHeight)
      return null;
    return (col, row);
  }

  // ── Pointer handlers ─────────────────────────────

  void _handlePointerDown(PointerDownEvent event) {
    if (_spaceHeld) {
      _panStart = event.localPosition - _panOffset;
      return;
    }
    final ts = ref.read(tilemapProvider);
    final tile = _tileFromLocal(event.localPosition, ts);
    if (tile == null) return;
    final (col, row) = tile;
    final notifier = ref.read(tilemapProvider.notifier);

    switch (ts.activeTool) {
      case TilemapTool.stamp:
      case TilemapTool.eraser:
        notifier.beginStamp(col, row);
        break;
      case TilemapTool.bucket:
        notifier.bucketFill(col, row);
        break;
      case TilemapTool.eyedropper:
        notifier.pickTile(col, row);
        break;
    }
  }

  void _handlePointerMove(PointerMoveEvent event) {
    if (_spaceHeld && event.buttons != 0) {
      setState(() => _panOffset = event.localPosition - _panStart);
      return;
    }

    final ts = ref.read(tilemapProvider);
    final tile = _tileFromLocal(event.localPosition, ts);
    if (tile != _hoverTile) setState(() => _hoverTile = tile);

    if (event.buttons != 0 && tile != null) {
      final (col, row) = tile;
      ref.read(tilemapProvider.notifier).continueStamp(col, row);
    }
  }

  void _handlePointerUp(PointerUpEvent event) {
    ref.read(tilemapProvider.notifier).endStroke();
  }

  void _handleScroll(PointerSignalEvent event) {
    if (event is PointerScaleEvent) {
      // Pinch-to-zoom on trackpad
      _pinchAccum += (event.scale - 1.0);
      if (_pinchAccum > 0.1) {
        ref.read(tilemapProvider.notifier).zoomIn();
        _pinchAccum = 0.0;
      } else if (_pinchAccum < -0.1) {
        ref.read(tilemapProvider.notifier).zoomOut();
        _pinchAccum = 0.0;
      }
      return;
    }
    if (event is PointerScrollEvent) {
      if (HardwareKeyboard.instance.isMetaPressed) {
        // Cmd + scroll → zoom
        final notifier = ref.read(tilemapProvider.notifier);
        if (event.scrollDelta.dy < 0) {
          notifier.zoomIn();
        } else {
          notifier.zoomOut();
        }
      } else {
        // Scroll → pan
        setState(() {
          _panOffset -= event.scrollDelta;
        });
      }
    }
  }

  MouseCursor _cursorForTool(TilemapTool tool) {
    if (_spaceHeld) return SystemMouseCursors.grab;
    return switch (tool) {
      TilemapTool.stamp => SystemMouseCursors.precise,
      TilemapTool.eraser => SystemMouseCursors.precise,
      TilemapTool.bucket => SystemMouseCursors.click,
      TilemapTool.eyedropper => SystemMouseCursors.click,
    };
  }

  @override
  Widget build(BuildContext context) {
    final ts = ref.watch(tilemapProvider);

    return LayoutBuilder(
      builder: (context, constraints) {
        final cellSize = ts.tilePixelSize * ts.zoomLevel;
        final totalW = ts.gridWidth * cellSize;
        final totalH = ts.gridHeight * cellSize;
        _centerX = (constraints.maxWidth - totalW) / 2;
        _centerY = (constraints.maxHeight - totalH) / 2;

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
            final notifier = ref.read(tilemapProvider.notifier);
            final meta = HardwareKeyboard.instance.isMetaPressed;
            switch (event.logicalKey) {
              case LogicalKeyboardKey.keyT:
                notifier.setTool(TilemapTool.stamp);
                return KeyEventResult.handled;
              case LogicalKeyboardKey.keyE:
                notifier.setTool(TilemapTool.eraser);
                return KeyEventResult.handled;
              case LogicalKeyboardKey.keyG:
                notifier.setTool(TilemapTool.bucket);
                return KeyEventResult.handled;
              case LogicalKeyboardKey.keyI:
                notifier.setTool(TilemapTool.eyedropper);
                return KeyEventResult.handled;
              case LogicalKeyboardKey.keyH:
                notifier.toggleGrid();
                return KeyEventResult.handled;
              case LogicalKeyboardKey.keyZ:
                if (meta) {
                  if (HardwareKeyboard.instance.isShiftPressed) {
                    notifier.redo();
                  } else {
                    notifier.undo();
                  }
                  return KeyEventResult.handled;
                }
                return KeyEventResult.ignored;
              // Zoom shortcuts
              case LogicalKeyboardKey.equal:
              case LogicalKeyboardKey.numpadAdd:
                notifier.zoomIn();
                return KeyEventResult.handled;
              case LogicalKeyboardKey.minus:
              case LogicalKeyboardKey.numpadSubtract:
                notifier.zoomOut();
                return KeyEventResult.handled;
              // Cmd+0 → reset zoom and pan
              case LogicalKeyboardKey.digit0:
                if (meta) {
                  notifier.resetZoom();
                  setState(() => _panOffset = Offset.zero);
                  return KeyEventResult.handled;
                }
                return KeyEventResult.ignored;
              default:
                return KeyEventResult.ignored;
            }
          },
          child: Listener(
            onPointerSignal: _handleScroll,
            child: MouseRegion(
              cursor: _cursorForTool(ts.activeTool),
              onHover: (event) {
                final tile = _tileFromLocal(event.localPosition, ts);
                if (tile != _hoverTile) setState(() => _hoverTile = tile);
              },
              child: Listener(
                onPointerDown: _handlePointerDown,
                onPointerMove: _handlePointerMove,
                onPointerUp: _handlePointerUp,
                child: CustomPaint(
                  size: Size(constraints.maxWidth, constraints.maxHeight),
                  painter: _TilemapPainter(
                    tilemapState: ts,
                    tileImages: _tileImages,
                    centerX: _centerX + _panOffset.dx,
                    centerY: _centerY + _panOffset.dy,
                    hoverTile: _hoverTile,
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}

// ── Tilemap Painter ──────────────────────────────────

class _TilemapPainter extends CustomPainter {
  _TilemapPainter({
    required this.tilemapState,
    required this.tileImages,
    required this.centerX,
    required this.centerY,
    this.hoverTile,
  });

  final TilemapState tilemapState;
  final Map<String, ui.Image> tileImages;
  final double centerX;
  final double centerY;
  final (int, int)? hoverTile;

  static const _checkerLight = Color(0xFF3a3a3a);
  static const _checkerDark = Color(0xFF2e2e2e);
  static const _gridColor = Color(0x33ffffff);
  static const _hoverColor = Color(0x22ffffff);
  static const _emptyColor = Color(0xFF333333);

  @override
  void paint(Canvas canvas, Size size) {
    final ts = tilemapState;
    final cellSize = ts.tilePixelSize * ts.zoomLevel;
    final w = ts.gridWidth;
    final h = ts.gridHeight;

    canvas.save();
    canvas.translate(centerX, centerY);

    final paint = Paint();

    // Draw cells
    for (var row = 0; row < h; row++) {
      for (var col = 0; col < w; col++) {
        final rect = Rect.fromLTWH(
          col * cellSize,
          row * cellSize,
          cellSize,
          cellSize,
        );
        final tileName = ts.cells[row][col];

        if (tileName != null && tileImages.containsKey(tileName)) {
          // Draw tile image scaled to cell
          final img = tileImages[tileName]!;
          final src = Rect.fromLTWH(
            0,
            0,
            img.width.toDouble(),
            img.height.toDouble(),
          );
          canvas.drawImageRect(img, src, rect, paint);
        } else {
          // Checkerboard for empty cells
          final isLight = (col + row) % 2 == 0;
          paint.color = isLight ? _checkerLight : _checkerDark;
          canvas.drawRect(rect, paint);

          // If tile name is set but image not loaded, show placeholder
          if (tileName != null) {
            paint.color = _emptyColor;
            canvas.drawRect(rect.deflate(1), paint);
          }
        }
      }
    }

    // Grid overlay
    if (ts.showGrid) {
      final gridPaint = Paint()
        ..color = _gridColor
        ..strokeWidth = 0.5;

      for (var col = 0; col <= w; col++) {
        canvas.drawLine(
          Offset(col * cellSize, 0),
          Offset(col * cellSize, h * cellSize),
          gridPaint,
        );
      }
      for (var row = 0; row <= h; row++) {
        canvas.drawLine(
          Offset(0, row * cellSize),
          Offset(w * cellSize, row * cellSize),
          gridPaint,
        );
      }
    }

    // Hover highlight
    if (hoverTile != null) {
      final (hCol, hRow) = hoverTile!;
      paint.color = _hoverColor;
      canvas.drawRect(
        Rect.fromLTWH(hCol * cellSize, hRow * cellSize, cellSize, cellSize),
        paint,
      );
    }

    canvas.restore();
  }

  @override
  bool shouldRepaint(covariant _TilemapPainter oldDelegate) {
    return tilemapState != oldDelegate.tilemapState ||
        tileImages != oldDelegate.tileImages ||
        centerX != oldDelegate.centerX ||
        centerY != oldDelegate.centerY ||
        hoverTile != oldDelegate.hoverTile;
  }
}
