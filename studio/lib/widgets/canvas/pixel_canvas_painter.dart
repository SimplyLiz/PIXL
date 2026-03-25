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
  });

  final CanvasState canvasState;
  final double pixelSize;
  final Offset? hoverPixel;
  /// Optional blueprint landmarks: list of {name, x, y, w?, h?} for overlay guides.
  final List<Map<String, dynamic>>? blueprintLandmarks;

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
      blueprintLandmarks != oldDelegate.blueprintLandmarks;
}
