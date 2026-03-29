import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' as ui show Color, Image, ImageByteFormat, decodeImageFromList;

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../services/pixl_backend.dart';

/// Backend connection status.
enum BackendStatus {
  disconnected,
  connecting,
  connected,
  error,
}

/// State exposed by the backend provider.
class BackendState {
  const BackendState({
    this.status = BackendStatus.disconnected,
    this.errorMessage,
    this.sessionTheme,
    this.sessionPalette,
    this.tiles = const [],
    this.stamps = const [],
    this.knowledgeAvailable = false,
    this.knowledgeEnabled = true,
  });

  final BackendStatus status;
  final String? errorMessage;
  final String? sessionTheme;
  final String? sessionPalette;
  final List<TileInfo> tiles;
  final List<String> stamps;
  /// Whether the knowledge base is loaded on the backend.
  final bool knowledgeAvailable;
  /// Whether to inject knowledge into generation prompts.
  final bool knowledgeEnabled;

  bool get isConnected => status == BackendStatus.connected;

  /// Copy with new values. Pass [clearError] = true to explicitly clear
  /// errorMessage; otherwise it is preserved from the current state.
  BackendState copyWith({
    BackendStatus? status,
    String? errorMessage,
    bool clearError = false,
    String? sessionTheme,
    String? sessionPalette,
    List<TileInfo>? tiles,
    List<String>? stamps,
    bool? knowledgeAvailable,
    bool? knowledgeEnabled,
  }) {
    return BackendState(
      status: status ?? this.status,
      errorMessage: clearError ? null : (errorMessage ?? this.errorMessage),
      sessionTheme: sessionTheme ?? this.sessionTheme,
      sessionPalette: sessionPalette ?? this.sessionPalette,
      tiles: tiles ?? this.tiles,
      stamps: stamps ?? this.stamps,
      knowledgeAvailable: knowledgeAvailable ?? this.knowledgeAvailable,
      knowledgeEnabled: knowledgeEnabled ?? this.knowledgeEnabled,
    );
  }
}

/// Info about a tile from the backend.
class TileInfo {
  const TileInfo({
    required this.name,
    this.size,
    this.previewBase64,
    this.edgeClasses,
    this.tags = const [],
  });

  final String name;
  final String? size;
  final String? previewBase64;
  final Map<String, String>? edgeClasses;
  final List<String> tags;

  Uint8List? get previewBytes =>
      previewBase64 != null ? base64Decode(previewBase64!) : null;
}

/// Validation result from the backend.
class ValidationReport {
  const ValidationReport({
    this.valid = false,
    this.errors = const [],
    this.warnings = const [],
    this.edgeCompat,
    this.paletteCompliant,
    this.sizeCorrect,
  });

  final bool valid;
  final List<String> errors;
  final List<String> warnings;
  final bool? edgeCompat;
  final bool? paletteCompliant;
  final bool? sizeCorrect;

  factory ValidationReport.fromJson(Map<String, dynamic> json) {
    return ValidationReport(
      valid: json['valid'] as bool? ?? false,
      errors: (json['errors'] as List<dynamic>?)
              ?.map((e) => e.toString())
              .toList() ??
          [],
      warnings: (json['warnings'] as List<dynamic>?)
              ?.map((e) => e.toString())
              .toList() ??
          [],
      edgeCompat: json['edge_compat'] as bool?,
      paletteCompliant: json['palette_compliant'] as bool?,
      sizeCorrect: json['size_correct'] as bool?,
    );
  }
}

class BackendNotifier extends StateNotifier<BackendState> {
  BackendNotifier() : super(const BackendState());

  final PixlBackend _backend = PixlBackend();

  PixlBackend get backend => _backend;

  @override
  void dispose() {
    _backend.stop();
    super.dispose();
  }

  /// Start the backend server and initialize session.
  /// Pass [model] and [adapter] to enable local LoRA inference on the backend.
  Future<void> connect({
    String? paxFile,
    String? model,
    String? adapter,
  }) async {
    state = state.copyWith(status: BackendStatus.connecting, clearError: true);

    final started = await _backend.start(
      paxFile: paxFile,
      model: model,
      adapter: adapter,
    );
    if (!started) {
      // Server might already be running externally
      final healthy = await _backend.isHealthy;
      if (!healthy) {
        state = state.copyWith(
          status: BackendStatus.error,
          errorMessage: 'Could not connect to PIXL engine on port ${_backend.port}',
        );
        return;
      }
    }

    // Initialize session
    try {
      final session = await _backend.sessionStart();
      if (session.containsKey('error')) {
        state = state.copyWith(
          status: BackendStatus.error,
          errorMessage: session['error'] as String,
        );
        return;
      }

      state = state.copyWith(
        status: BackendStatus.connected,
        clearError: true,
        sessionTheme: session['theme'] as String?,
        sessionPalette: session['palette'] as String?,
      );

      // Load tiles and stamps
      await refreshTiles();
      await refreshStamps();
    } catch (e) {
      state = state.copyWith(
        status: BackendStatus.error,
        errorMessage: 'Session init failed: $e',
      );
    }
  }

  /// Disconnect and stop the server.
  void disconnect() {
    _backend.stop();
    state = const BackendState(status: BackendStatus.disconnected);
  }

  /// Refresh the tile list from the backend.
  Future<void> refreshTiles() async {
    final resp = await _backend.listTiles();
    if (resp.containsKey('error')) return;

    final tileList = <TileInfo>[];
    final tiles = resp['tiles'] as List<dynamic>? ?? [];
    for (final t in tiles) {
      if (t is Map<String, dynamic>) {
        tileList.add(TileInfo(
          name: t['name'] as String? ?? '',
          size: t['size'] as String?,
          previewBase64: t['preview'] as String?,
          edgeClasses: (t['edge_class'] as Map<String, dynamic>?)
              ?.map((k, v) => MapEntry(k, v.toString())),
          tags: (t['tags'] as List<dynamic>?)
                  ?.map((e) => e.toString())
                  .toList() ??
              [],
        ));
      }
    }
    state = state.copyWith(tiles: tileList);
  }

  /// Refresh stamps list.
  Future<void> refreshStamps() async {
    final resp = await _backend.listStamps();
    if (resp.containsKey('error')) return;

    final stamps = (resp['stamps'] as List<dynamic>?)
            ?.map((e) => e.toString())
            .toList() ??
        [];
    state = state.copyWith(stamps: stamps);
  }

  /// Create a tile via the backend.
  Future<Map<String, dynamic>> createTile({
    required String name,
    required String palette,
    required String size,
    required String grid,
    Map<String, String>? edgeClass,
    String? symmetry,
    List<String>? tags,
  }) async {
    final resp = await _backend.createTile(
      name: name,
      palette: palette,
      size: size,
      grid: grid,
      edgeClass: edgeClass,
      symmetry: symmetry,
      tags: tags,
    );
    if (!resp.containsKey('error')) {
      await refreshTiles();
    }
    return resp;
  }

  /// Render a tile, returns base64 PNG.
  Future<String?> renderTile(String name, {int scale = 16}) async {
    final resp = await _backend.renderTile(name, scale: scale);
    if (resp.containsKey('error')) return null;
    return resp['png'] as String? ?? resp['preview'] as String?;
  }

  /// Fetch a tile's pixel data at 1:1 scale for canvas editing.
  ///
  /// Returns decoded RGBA pixels + dimensions, or null on failure.
  Future<({List<ui.Color?> pixels, int width, int height})?> getTilePixels(
    String name,
  ) async {
    final resp = await _backend.renderTile(name, scale: 1);
    if (resp.containsKey('error')) return null;

    final b64 = resp['preview_b64'] as String?;
    final sizeStr = resp['size'] as String? ?? '16x16';
    if (b64 == null) return null;

    final parts = sizeStr.split('x');
    final tileW = int.tryParse(parts[0]) ?? 16;
    final tileH = int.tryParse(parts.length > 1 ? parts[1] : '16') ?? 16;

    // Decode PNG to raw RGBA bytes via dart:ui.
    final pngBytes = base64Decode(b64);
    final image = await _decodePng(pngBytes);
    if (image == null) return null;

    final byteData = await image.toByteData(
      format: ui.ImageByteFormat.rawRgba,
    );
    image.dispose();
    if (byteData == null) return null;

    final rgba = byteData.buffer.asUint8List();
    final pixels = <ui.Color?>[];
    for (var i = 0; i < rgba.length; i += 4) {
      final a = rgba[i + 3];
      if (a == 0) {
        pixels.add(null);
      } else {
        pixels.add(ui.Color.fromARGB(a, rgba[i], rgba[i + 1], rgba[i + 2]));
      }
    }

    return (pixels: pixels, width: tileW, height: tileH);
  }

  static Future<ui.Image?> _decodePng(Uint8List bytes) async {
    final completer = Completer<ui.Image?>();
    ui.decodeImageFromList(bytes, (image) => completer.complete(image));
    return completer.future;
  }

  /// Validate the session.
  Future<ValidationReport> validate({bool checkEdges = false}) async {
    final resp = await _backend.validate(checkEdges: checkEdges);
    if (resp.containsKey('error')) {
      return ValidationReport(
        valid: false,
        errors: [resp['error'] as String],
      );
    }
    return ValidationReport.fromJson(resp);
  }

  /// Generate a tile using the local LoRA model (server-side).
  Future<Map<String, dynamic>> generateTile({
    required String name,
    required String prompt,
    String size = '16x16',
    String? palette,
  }) async {
    final resp = await _backend.generateTile(
      name: name,
      prompt: prompt,
      size: size,
      palette: palette,
    );
    if (resp['ok'] == true) {
      await refreshTiles();
    }
    return resp;
  }

  /// Toggle knowledge base injection on/off.
  void setKnowledgeEnabled(bool enabled) {
    state = state.copyWith(knowledgeEnabled: enabled);
  }

  /// Get enriched generation context.
  Future<Map<String, dynamic>> getGenerationContext({
    required String prompt,
    String type = 'tile',
    String size = '16x16',
  }) async {
    final ctx = await _backend.generateContext(
      prompt: prompt,
      type: type,
      size: size,
      knowledgeEnabled: state.knowledgeEnabled,
    );
    // Update knowledge availability from backend response
    final kb = ctx['knowledge'] as Map<String, dynamic>?;
    if (kb != null) {
      state = state.copyWith(
        knowledgeAvailable: kb['available'] as bool? ?? false,
      );
    }
    return ctx;
  }

  /// Delete a tile.
  Future<void> deleteTile(String name) async {
    await _backend.deleteTile(name);
    await refreshTiles();
  }

  /// Load PAX source into session.
  ///
  /// On success, updates [sessionTheme] / [sessionPalette] from the engine
  /// response so the UI can sync its palette panel.
  Future<Map<String, dynamic>> loadSource(String source) async {
    final resp = await _backend.loadSource(source);
    if (!resp.containsKey('error')) {
      state = state.copyWith(
        sessionTheme: resp['active_theme'] as String?,
        sessionPalette: resp['active_palette'] as String?,
      );
      await refreshTiles();
    }
    return resp;
  }

  /// Pack atlas.
  Future<Map<String, dynamic>> packAtlas({
    int columns = 8,
    int padding = 1,
    int scale = 1,
  }) async {
    return _backend.packAtlas(
      columns: columns,
      padding: padding,
      scale: scale,
    );
  }

  /// Get .pax source file.
  Future<String?> getPaxSource() async {
    final resp = await _backend.getFile();
    if (resp.containsKey('error')) return null;
    return resp['source'] as String?;
  }

  /// Get .pax source in compact PAX-L format (~40% fewer tokens).
  Future<String?> getPaxlSource() async {
    final resp = await _backend.getFilePaxl();
    if (resp.containsKey('error')) return null;
    return resp['paxl_source'] as String?;
  }

  /// Rate a tile aesthetically (1-5 on readability, appeal, consistency).
  Future<Map<String, dynamic>?> rateTile(String name) async {
    final resp = await _backend.rateTile(name);
    if (resp.containsKey('error')) return null;
    return resp;
  }

  /// Check if the tileset is sub-complete (WFC contradiction-free).
  Future<Map<String, dynamic>?> checkSubcomplete() async {
    final resp = await _backend.checkSubcomplete();
    if (resp.containsKey('error')) return null;
    return resp;
  }

  /// Generate a complete Wang tileset for terrain transitions.
  Future<Map<String, dynamic>?> generateWang({
    required String terrainA,
    required String terrainB,
    String method = 'dual_grid',
    int size = 16,
  }) async {
    final resp = await _backend.generateWang(
      terrainA: terrainA,
      terrainB: terrainB,
      method: method,
      size: size,
    );
    if (resp.containsKey('error')) return null;
    await refreshTiles();
    return resp;
  }
}

final backendProvider =
    StateNotifierProvider<BackendNotifier, BackendState>(
  (ref) => BackendNotifier(),
);
