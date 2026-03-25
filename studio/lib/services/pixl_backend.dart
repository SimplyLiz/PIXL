import 'dart:convert';
import 'dart:io';
import 'package:http/http.dart' as http;

/// Service that communicates with the PIXL Rust engine via HTTP API.
/// The engine runs as `pixl serve` on localhost:3742.
class PixlBackend {
  PixlBackend({
    this.port = 3742,
    String? binaryPath,
  }) : _binaryPath = binaryPath ?? _findBinary();

  final int port;
  final String _binaryPath;
  Process? _serverProcess;

  String get _baseUrl => 'http://127.0.0.1:$port';

  static String _findBinary() {
    // Resolve the project root — works regardless of working directory.
    // When running from `flutter run` inside studio/, the script is in
    // studio/lib/. When running a built app, fall back to cwd-relative
    // paths and then PATH lookup.
    final scriptDir = File(Platform.script.toFilePath()).parent.path;

    // Walk up from studio/lib/ or studio/ to find the project root
    Directory? projectRoot;
    var dir = Directory(scriptDir);
    for (var i = 0; i < 6; i++) {
      if (File('${dir.path}/tool/Cargo.toml').existsSync()) {
        projectRoot = dir;
        break;
      }
      dir = dir.parent;
    }

    final candidates = <String>[
      // Absolute paths from project root (most reliable)
      if (projectRoot != null) ...[
        '${projectRoot.path}/tool/target/release/pixl',
        '${projectRoot.path}/tool/target/debug/pixl',
      ],
      // Relative paths (work when cwd is studio/ or project root)
      '../tool/target/release/pixl',
      '../tool/target/debug/pixl',
      'tool/target/release/pixl',
      'tool/target/debug/pixl',
      // PATH lookup
      'pixl',
    ];

    for (final c in candidates) {
      if (File(c).existsSync()) return c;
    }
    return 'pixl';
  }

  /// Start the backend server. Call this on app launch.
  /// Pass [model] and [adapter] to enable local LoRA inference.
  Future<bool> start({
    String? paxFile,
    String? model,
    String? adapter,
  }) async {
    try {
      final args = ['serve', '--port', '$port'];
      if (paxFile != null) {
        args.addAll(['--file', paxFile]);
      }
      if (model != null && model.isNotEmpty) {
        args.addAll(['--model', model]);
      }
      if (adapter != null && adapter.isNotEmpty) {
        args.addAll(['--adapter', adapter]);
      }

      // ignore: avoid_print
      print('[pixl] starting engine: $_binaryPath ${args.join(' ')}');
      _serverProcess = await Process.start(_binaryPath, args);

      // Forward stderr for diagnostics
      _serverProcess!.stderr.transform(const SystemEncoding().decoder).listen(
        // ignore: avoid_print
        (line) => print('[pixl-engine] $line'),
      );

      // Wait for server to be ready
      for (var i = 0; i < 30; i++) {
        await Future.delayed(const Duration(milliseconds: 200));
        try {
          final resp = await http.get(Uri.parse('$_baseUrl/health'));
          if (resp.statusCode == 200) {
            // ignore: avoid_print
            print('[pixl] engine ready on port $port');
            return true;
          }
        } catch (_) {}
      }
      // ignore: avoid_print
      print('[pixl] engine failed to start within 6s (binary: $_binaryPath)');
      return false;
    } catch (e) {
      // ignore: avoid_print
      print('[pixl] engine start error: $e');
      return false;
    }
  }

  /// Stop the backend server. Call this on app exit.
  void stop() {
    _serverProcess?.kill();
    _serverProcess = null;
  }

  /// Check if the server is running.
  Future<bool> get isHealthy async {
    try {
      final resp = await http.get(Uri.parse('$_baseUrl/health'));
      return resp.statusCode == 200;
    } catch (_) {
      return false;
    }
  }

  // ── Session ────────────────────────────────────────────

  /// Start or get current session info.
  Future<Map<String, dynamic>> sessionStart() async {
    return _post('/api/session', {});
  }

  /// Load a .pax source into the session.
  Future<Map<String, dynamic>> loadSource(String paxSource) async {
    return _post('/api/load', {'source': paxSource});
  }

  // ── Palette & Theme ────────────────────────────────────

  /// Get palette symbols for a theme.
  Future<Map<String, dynamic>> getPalette(String theme) async {
    return _post('/api/palette', {'theme': theme});
  }

  /// List available themes.
  Future<Map<String, dynamic>> listThemes() async {
    return _get('/api/themes');
  }

  /// List available stamps.
  Future<Map<String, dynamic>> listStamps() async {
    return _get('/api/stamps');
  }

  // ── Tile CRUD ──────────────────────────────────────────

  /// Create a tile. Returns validation + 16x preview + edge context.
  Future<Map<String, dynamic>> createTile({
    required String name,
    required String palette,
    required String size,
    required String grid,
    Map<String, String>? edgeClass,
    String? symmetry,
    List<String>? tags,
  }) async {
    return _post('/api/tile/create', {
      'name': name,
      'palette': palette,
      'size': size,
      'grid': grid,
      if (edgeClass != null) 'edge_class': edgeClass,
      if (symmetry != null) 'symmetry': symmetry,
      if (tags != null) 'tags': tags,
    });
  }

  /// Render a tile to PNG (base64).
  Future<Map<String, dynamic>> renderTile(String name, {int scale = 16}) async {
    return _post('/api/tile/render', {'name': name, 'scale': scale});
  }

  /// Delete a tile.
  Future<Map<String, dynamic>> deleteTile(String name) async {
    return _post('/api/tile/delete', {'name': name});
  }

  /// Check if two tiles can be adjacent.
  Future<Map<String, dynamic>> checkEdgePair(
    String tileA,
    String direction,
    String tileB,
  ) async {
    return _post('/api/tile/edge-check', {
      'tile_a': tileA,
      'direction': direction,
      'tile_b': tileB,
    });
  }

  /// List all tiles.
  Future<Map<String, dynamic>> listTiles() async {
    return _get('/api/tiles');
  }

  // ── Validation ─────────────────────────────────────────

  /// Validate the current session state.
  Future<Map<String, dynamic>> validate({bool checkEdges = false}) async {
    return _post('/api/validate', {'check_edges': checkEdges});
  }

  // ── Generation ─────────────────────────────────────────

  /// Get enriched context for AI generation.
  /// Returns system_prompt + user_prompt for the Studio to send to Claude.
  Future<Map<String, dynamic>> generateContext({
    required String prompt,
    String type = 'tile',
    String size = '16x16',
    bool knowledgeEnabled = true,
  }) async {
    return _post('/api/generate/context', {
      'prompt': prompt,
      'type': type,
      'size': size,
      'knowledge_enabled': knowledgeEnabled,
    });
  }

  /// Generate a tile using the local LoRA model (server-side).
  /// Returns the created tile with preview, edges, and generation metadata.
  Future<Map<String, dynamic>> generateTile({
    required String name,
    required String prompt,
    String size = '16x16',
    String? palette,
  }) async {
    return _post('/api/generate/tile', {
      'name': name,
      'prompt': prompt,
      'size': size,
      if (palette != null) 'palette': palette,
    });
  }

  /// Generate a map from narrative predicates.
  Future<Map<String, dynamic>> narrateMap({
    required int width,
    required int height,
    required List<String> rules,
    int seed = 42,
  }) async {
    return _post('/api/narrate', {
      'width': width,
      'height': height,
      'rules': rules,
      'seed': seed,
    });
  }

  // ── Style ──────────────────────────────────────────────

  /// Learn style from reference tiles.
  Future<Map<String, dynamic>> learnStyle({List<String>? tiles}) async {
    return _post('/api/style/learn', {
      if (tiles != null) 'tiles': tiles,
    });
  }

  /// Score a tile against the style latent.
  Future<Map<String, dynamic>> checkStyle(String tileName) async {
    return _post('/api/style/check', {'name': tileName});
  }

  // ── Blueprint ──────────────────────────────────────────

  /// Get anatomy blueprint for character sprites.
  Future<Map<String, dynamic>> getBlueprint({
    int width = 32,
    int height = 48,
    String model = 'humanoid_chibi',
  }) async {
    return _post('/api/blueprint', {
      'width': width,
      'height': height,
      'model': model,
    });
  }

  // ── Export ─────────────────────────────────────────────

  /// Pack atlas (base64 PNG + JSON metadata).
  Future<Map<String, dynamic>> packAtlas({
    int columns = 8,
    int padding = 1,
    int scale = 1,
  }) async {
    return _post('/api/atlas/pack', {
      'columns': columns,
      'padding': padding,
      'scale': scale,
    });
  }

  /// Render sprite as animated GIF (base64).
  Future<Map<String, dynamic>> renderSpriteGif({
    required String spriteset,
    required String sprite,
    int scale = 8,
  }) async {
    return _post('/api/sprite/gif', {
      'spriteset': spriteset,
      'sprite': sprite,
      'scale': scale,
    });
  }

  /// Get the full .pax source.
  Future<Map<String, dynamic>> getFile() async {
    return _get('/api/file');
  }

  // ── Feedback ─────────────────────────────────────────

  /// Record accept/reject/edit feedback for a tile.
  Future<Map<String, dynamic>> recordFeedback({
    required String name,
    required String action,
    String? rejectReason,
  }) async {
    return _post('/api/feedback', {
      'name': name,
      'action': action,
      if (rejectReason != null) 'reject_reason': rejectReason,
    });
  }

  /// Get feedback statistics.
  Future<Map<String, dynamic>> feedbackStats() async {
    return _get('/api/feedback/stats');
  }

  /// Get feedback constraints for generation.
  Future<Map<String, dynamic>> feedbackConstraints() async {
    return _get('/api/feedback/constraints');
  }

  // ── Training ────────────────────────────────────────────

  /// Get training data statistics.
  Future<Map<String, dynamic>> trainingStats() async {
    return _get('/api/training/stats');
  }

  /// Export accepted tiles as training JSONL.
  Future<Map<String, dynamic>> exportTraining({String? path}) async {
    return _post('/api/training/export', {
      if (path != null) 'path': path,
    });
  }

  // ── New & Export ──────────────────────────────────────

  /// Create a new PAX file from a built-in theme template.
  /// Returns the PAX source string which can be passed to [loadSource].
  Future<Map<String, dynamic>> newFromTemplate(String theme) async {
    return _post('/api/new', {'theme': theme});
  }

  /// Export the current session to a game engine format.
  /// [format] is one of: texturepacker, tiled, godot.
  /// [outDir] is the absolute path to write export files.
  Future<Map<String, dynamic>> exportToEngine({
    required String format,
    required String outDir,
  }) async {
    return _post('/api/export', {
      'format': format,
      'out_dir': outDir,
    });
  }

  // ── Generic ────────────────────────────────────────────

  /// Call any tool by name.
  Future<Map<String, dynamic>> callTool(
    String toolName,
    Map<String, dynamic> args,
  ) async {
    return _post('/api/tool', {'tool': toolName, 'args': args});
  }

  // ── HTTP helpers ───────────────────────────────────────

  static const _timeout = Duration(seconds: 30);

  Future<Map<String, dynamic>> _get(String path) async {
    try {
      final resp = await http
          .get(Uri.parse('$_baseUrl$path'))
          .timeout(_timeout);
      return jsonDecode(resp.body) as Map<String, dynamic>;
    } catch (e) {
      return {'error': 'HTTP error: $e'};
    }
  }

  Future<Map<String, dynamic>> _post(
    String path,
    Map<String, dynamic> body,
  ) async {
    try {
      final resp = await http
          .post(
            Uri.parse('$_baseUrl$path'),
            headers: {'Content-Type': 'application/json'},
            body: jsonEncode(body),
          )
          .timeout(_timeout);
      return jsonDecode(resp.body) as Map<String, dynamic>;
    } catch (e) {
      return {'error': 'HTTP error: $e'};
    }
  }
}
