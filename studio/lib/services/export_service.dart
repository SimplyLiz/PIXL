import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:file_picker/file_picker.dart';
import '../models/pixel_canvas.dart';

/// Export service for saving tiles, PAX source, and atlases.
class ExportService {
  /// Export the current canvas as a scaled PNG.
  static Future<bool> exportCanvasPng({
    required CanvasState canvasState,
    int scale = 4,
  }) async {
    final path = await FilePicker.platform.saveFile(
      dialogTitle: 'Export PNG',
      fileName: 'tile_${canvasState.canvasSize.label}.png',
      type: FileType.custom,
      allowedExtensions: ['png'],
    );
    if (path == null) return false;

    final w = canvasState.width;
    final h = canvasState.height;
    final scaledW = w * scale;
    final scaledH = h * scale;

    // Build pixel buffer
    final pixels = Uint32List(scaledW * scaledH);

    // Composite layers
    for (var y = 0; y < h; y++) {
      for (var x = 0; x < w; x++) {
        ui.Color? color;
        // Bottom-up layer compositing
        for (final layer in canvasState.layers) {
          if (!layer.visible) continue;
          final layerColor = layer.pixels[y * w + x];
          if (layerColor != null) {
            color = layerColor;
          }
        }

        if (color != null) {
          final argb = color.toARGB32();
          // Convert ARGB → RGBA byte order for PixelFormat.rgba8888
          // Uint32 is stored in native byte order; rgba8888 expects
          // the 32-bit value as 0xRRGGBBAA on big-endian conceptual layout,
          // but Uint32List on little-endian stores bytes as AA BB GG RR.
          // So we pack as ABGR in the Uint32 to get RGBA bytes in memory.
          final a = (argb >> 24) & 0xFF;
          final r = (argb >> 16) & 0xFF;
          final g = (argb >> 8) & 0xFF;
          final b = argb & 0xFF;
          final rgba = (a << 24) | (b << 16) | (g << 8) | r;

          // Fill scaled block
          for (var sy = 0; sy < scale; sy++) {
            for (var sx = 0; sx < scale; sx++) {
              pixels[(y * scale + sy) * scaledW + (x * scale + sx)] = rgba;
            }
          }
        }
      }
    }

    // Encode to PNG using dart:ui
    final completer = ui.ImmutableBuffer.fromUint8List(
      pixels.buffer.asUint8List(),
    );
    final buffer = await completer;
    final descriptor = ui.ImageDescriptor.raw(
      buffer,
      width: scaledW,
      height: scaledH,
      pixelFormat: ui.PixelFormat.rgba8888,
    );
    final codec = await descriptor.instantiateCodec();
    final frame = await codec.getNextFrame();
    final byteData = await frame.image.toByteData(
      format: ui.ImageByteFormat.png,
    );

    if (byteData == null) return false;

    await File(path).writeAsBytes(byteData.buffer.asUint8List());
    return true;
  }

  /// Save PAX source to file.
  static Future<bool> savePaxSource(String source) async {
    final path = await FilePicker.platform.saveFile(
      dialogTitle: 'Save PAX Source',
      fileName: 'tileset.pax',
      type: FileType.custom,
      allowedExtensions: ['pax'],
    );
    if (path == null) return false;

    await File(path).writeAsString(source);
    return true;
  }

  /// Save atlas PNG from base64.
  static Future<bool> saveAtlasPng(String base64Png) async {
    final path = await FilePicker.platform.saveFile(
      dialogTitle: 'Save Atlas',
      fileName: 'atlas.png',
      type: FileType.custom,
      allowedExtensions: ['png'],
    );
    if (path == null) return false;

    await File(path).writeAsBytes(base64Decode(base64Png));
    return true;
  }
}
