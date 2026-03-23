import 'package:flutter/material.dart';

import '../../models/pixel_canvas.dart';

/// CustomPainter that renders the pixel grid with checkerboard transparency,
/// layer compositing, and optional grid overlay.
class PixelCanvasPainter extends CustomPainter {
  PixelCanvasPainter({
    required this.canvasState,
    required this.pixelSize,
    this.hoverPixel,
  });

  final CanvasState canvasState;
  final double pixelSize;
  final Offset? hoverPixel;

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

    // Composite layers bottom-up
    final pixelPaint = Paint()..style = PaintingStyle.fill;
    for (final layer in canvasState.layers) {
      if (!layer.visible) continue;
      for (var y = 0; y < h; y++) {
        for (var x = 0; x < w; x++) {
          final color = layer.pixels[y * w + x];
          if (color != null) {
            pixelPaint.color = color;
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
      hoverPixel != oldDelegate.hoverPixel;
}
