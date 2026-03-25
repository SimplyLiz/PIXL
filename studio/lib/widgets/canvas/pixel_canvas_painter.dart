import 'dart:ui' as ui;

import 'package:flutter/material.dart';

import '../../models/pixel_canvas.dart';

/// CustomPainter that renders the pixel grid with checkerboard transparency,
/// layer compositing, and optional grid overlay.
class PixelCanvasPainter extends CustomPainter {
  PixelCanvasPainter({
    required this.canvasState,
    required this.pixelSize,
    this.hoverPixel,
    this.blueprintLandmarks,
    this.shapePreview,
    this.selection,
    this.referenceImage,
    this.referenceOpacity = 0.3,
  });

  final CanvasState canvasState;
  final double pixelSize;
  final Offset? hoverPixel;
  final List<Map<String, dynamic>>? blueprintLandmarks;
  final (DrawingTool, (int, int), (int, int), bool)? shapePreview;
  final SelectionState? selection;
  /// Optional reference image overlay.
  final ui.Image? referenceImage;
  final double referenceOpacity;

  static const _checkerLight = Color(0xFF383838);
  static const _checkerDark = Color(0xFF2c2c2c);
  static const _gridColor = Color(0x30ffffff);
  static const _hoverColor = Color(0x60ffffff);

  @override
  void paint(Canvas canvas, Size size) {
    final w = canvasState.width;
    final h = canvasState.height;
    final ps = pixelSize;

    // Checkerboard background (transparency indicator)
    for (var y = 0; y < h; y++) {
      for (var x = 0; x < w; x++) {
        final isLight = (x + y) % 2 == 0;
        canvas.drawRect(
          Rect.fromLTWH(x * ps, y * ps, ps, ps),
          Paint()..color = isLight ? _checkerLight : _checkerDark,
        );
      }
    }

    // Reference image overlay (below pixels, above checkerboard)
    if (referenceImage != null) {
      final src = Rect.fromLTWH(0, 0,
        referenceImage!.width.toDouble(), referenceImage!.height.toDouble());
      final dst = Rect.fromLTWH(0, 0, w * ps, h * ps);
      canvas.drawImageRect(
        referenceImage!,
        src,
        dst,
        Paint()..color = Color.fromRGBO(255, 255, 255, referenceOpacity),
      );
    }

    // Composite layers bottom-up with opacity
    final pixelPaint = Paint()..style = PaintingStyle.fill;
    for (final layer in canvasState.layers) {
      if (!layer.visible || layer.opacity <= 0) continue;
      for (var y = 0; y < h; y++) {
        for (var x = 0; x < w; x++) {
          final color = layer.pixels[y * w + x];
          if (color != null) {
            pixelPaint.color = layer.opacity < 1.0
                ? color.withValues(alpha: color.a * layer.opacity)
                : color;
            canvas.drawRect(
              Rect.fromLTWH(x * ps, y * ps, ps, ps),
              pixelPaint,
            );
          }
        }
      }
    }

    // Grid overlay
    if (canvasState.showGrid && ps >= 4) {
      final gridPaint = Paint()
        ..color = _gridColor
        ..strokeWidth = 0.5;

      for (var x = 0; x <= w; x++) {
        canvas.drawLine(
          Offset(x * ps, 0),
          Offset(x * ps, h * ps),
          gridPaint,
        );
      }
      for (var y = 0; y <= h; y++) {
        canvas.drawLine(
          Offset(0, y * ps),
          Offset(w * ps, y * ps),
          gridPaint,
        );
      }
    }

    // Blueprint overlay
    if (blueprintLandmarks != null && blueprintLandmarks!.isNotEmpty) {
      final bpPaint = Paint()
        ..color = const Color(0x5500bcd4)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.0;
      final bpFill = Paint()
        ..color = const Color(0x2200bcd4)
        ..style = PaintingStyle.fill;
      final labelStyle = TextStyle(
        color: const Color(0xAA00bcd4),
        fontSize: (ps * 0.6).clamp(6, 10),
        fontWeight: FontWeight.w600,
      );

      for (final lm in blueprintLandmarks!) {
        final lx = (lm['x'] as num?)?.toDouble() ?? 0;
        final ly = (lm['y'] as num?)?.toDouble() ?? 0;
        final lw = (lm['w'] as num?)?.toDouble() ?? 1;
        final lh = (lm['h'] as num?)?.toDouble() ?? 1;
        final name = lm['name'] as String? ?? '';

        final rect = Rect.fromLTWH(lx * ps, ly * ps, lw * ps, lh * ps);
        canvas.drawRect(rect, bpFill);
        canvas.drawRect(rect, bpPaint);

        // Label
        if (name.isNotEmpty && ps >= 4) {
          final tp = TextPainter(
            text: TextSpan(text: name, style: labelStyle),
            textDirection: TextDirection.ltr,
          )..layout();
          tp.paint(canvas, Offset(rect.left + 1, rect.top - tp.height - 1));
        }
      }
    }

    // Shape preview (line/rect drag)
    if (shapePreview != null) {
      final (tool, start, end, shiftHeld) = shapePreview!;
      final previewPaint = Paint()
        ..color = const Color(0x99ffffff)
        ..style = PaintingStyle.fill;

      if (tool == DrawingTool.line) {
        // Bresenham preview
        var x0 = start.$1, y0 = start.$2;
        final x1 = end.$1, y1 = end.$2;
        var dx = (x1 - x0).abs();
        var dy = -(y1 - y0).abs();
        final sx = x0 < x1 ? 1 : -1;
        final sy = y0 < y1 ? 1 : -1;
        var err = dx + dy;
        while (true) {
          if (x0 >= 0 && x0 < w && y0 >= 0 && y0 < h) {
            canvas.drawRect(Rect.fromLTWH(x0 * ps, y0 * ps, ps, ps), previewPaint);
          }
          if (x0 == x1 && y0 == y1) break;
          final e2 = 2 * err;
          if (e2 >= dy) { err += dy; x0 += sx; }
          if (e2 <= dx) { err += dx; y0 += sy; }
        }
      } else if (tool == DrawingTool.rect) {
        final minX = start.$1 < end.$1 ? start.$1 : end.$1;
        final maxX = start.$1 > end.$1 ? start.$1 : end.$1;
        final minY = start.$2 < end.$2 ? start.$2 : end.$2;
        final maxY = start.$2 > end.$2 ? start.$2 : end.$2;
        if (shiftHeld) {
          // Filled rect preview
          canvas.drawRect(
            Rect.fromLTWH(minX * ps, minY * ps, (maxX - minX + 1) * ps, (maxY - minY + 1) * ps),
            previewPaint,
          );
        } else {
          // Outline preview
          for (var x = minX; x <= maxX; x++) {
            canvas.drawRect(Rect.fromLTWH(x * ps, minY * ps, ps, ps), previewPaint);
            canvas.drawRect(Rect.fromLTWH(x * ps, maxY * ps, ps, ps), previewPaint);
          }
          for (var y = minY + 1; y < maxY; y++) {
            canvas.drawRect(Rect.fromLTWH(minX * ps, y * ps, ps, ps), previewPaint);
            canvas.drawRect(Rect.fromLTWH(maxX * ps, y * ps, ps, ps), previewPaint);
          }
        }
      }
    }

    // Selection rectangle (dashed border)
    if (selection != null && selection!.hasSelection) {
      final sx = selection!.x * ps;
      final sy = selection!.y * ps;
      final sw = selection!.width * ps;
      final sh = selection!.height * ps;
      final selFill = Paint()
        ..color = const Color(0x1500bcd4)
        ..style = PaintingStyle.fill;
      final selStroke = Paint()
        ..color = const Color(0xCC00bcd4)
        ..style = PaintingStyle.stroke
        ..strokeWidth = 1.5;
      canvas.drawRect(Rect.fromLTWH(sx, sy, sw, sh), selFill);
      canvas.drawRect(Rect.fromLTWH(sx, sy, sw, sh), selStroke);
    }

    // Hover highlight
    if (hoverPixel != null) {
      final hx = hoverPixel!.dx.toInt();
      final hy = hoverPixel!.dy.toInt();
      if (hx >= 0 && hx < w && hy >= 0 && hy < h) {
        canvas.drawRect(
          Rect.fromLTWH(hx * ps, hy * ps, ps, ps),
          Paint()
            ..color = _hoverColor
            ..style = PaintingStyle.fill,
        );
      }
    }
  }

  @override
  bool shouldRepaint(PixelCanvasPainter oldDelegate) =>
      !identical(canvasState, oldDelegate.canvasState) ||
      pixelSize != oldDelegate.pixelSize ||
      hoverPixel != oldDelegate.hoverPixel ||
      blueprintLandmarks != oldDelegate.blueprintLandmarks ||
      shapePreview != oldDelegate.shapePreview ||
      selection != oldDelegate.selection ||
      referenceImage != oldDelegate.referenceImage ||
      referenceOpacity != oldDelegate.referenceOpacity;
}
