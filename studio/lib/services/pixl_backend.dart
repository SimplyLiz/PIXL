import 'dart:convert';
import 'dart:io';

/// Service that calls the PIXL Rust CLI for rendering, validation, etc.
/// The Rust binary is expected at `../tool/target/release/pixl-cli` or on PATH.
class PixlBackend {
  PixlBackend({String? binaryPath})
      : _binaryPath = binaryPath ?? _findBinary();

  final String _binaryPath;

  static String _findBinary() {
    // Look for the binary relative to the studio directory
    final candidates = [
      '../tool/target/release/pixl-cli',
      '../tool/target/debug/pixl-cli',
      'pixl-cli', // on PATH
    ];
    for (final c in candidates) {
      if (File(c).existsSync()) return c;
    }
    return 'pixl-cli'; // fallback to PATH
  }

  /// Validate a PAX source string.
  Future<ValidationResult> validate(String paxSource) async {
    final result = await _run(['validate', '--stdin'], stdin: paxSource);
    return ValidationResult(
      success: result.exitCode == 0,
      output: result.stdout,
      errors: result.stderr,
    );
  }

  /// Render a tile to PNG bytes (base64 encoded).
  Future<RenderResult> renderTile({
    required String paxFile,
    required String tileName,
    int scale = 4,
  }) async {
    final result = await _run([
      'render',
      paxFile,
      '--tile',
      tileName,
      '--scale',
      '$scale',
      '--out',
      '-', // stdout
    ]);
    if (result.exitCode != 0) {
      return RenderResult(success: false, error: result.stderr);
    }
    return RenderResult(success: true, pngBytes: result.stdoutBytes);
  }

  /// Run the PIXL CLI with given arguments.
  Future<_ProcessResult> _run(List<String> args, {String? stdin}) async {
    try {
      final process = await Process.start(_binaryPath, args);
      if (stdin != null) {
        process.stdin.write(stdin);
        await process.stdin.close();
      }
      final stdout = await process.stdout.transform(utf8.decoder).join();
      final stdoutBytes = await process.stdout.toList();
      final stderr = await process.stderr.transform(utf8.decoder).join();
      final exitCode = await process.exitCode;
      return _ProcessResult(
        exitCode: exitCode,
        stdout: stdout,
        stderr: stderr,
        stdoutBytes: stdoutBytes.expand((b) => b).toList(),
      );
    } catch (e) {
      return _ProcessResult(
        exitCode: -1,
        stdout: '',
        stderr: 'Failed to run pixl-cli: $e',
        stdoutBytes: [],
      );
    }
  }
}

class _ProcessResult {
  const _ProcessResult({
    required this.exitCode,
    required this.stdout,
    required this.stderr,
    required this.stdoutBytes,
  });
  final int exitCode;
  final String stdout;
  final String stderr;
  final List<int> stdoutBytes;
}

class ValidationResult {
  const ValidationResult({
    required this.success,
    this.output = '',
    this.errors = '',
  });
  final bool success;
  final String output;
  final String errors;
}

class RenderResult {
  const RenderResult({required this.success, this.pngBytes, this.error});
  final bool success;
  final List<int>? pngBytes;
  final String? error;
}
