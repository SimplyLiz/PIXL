import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../theme/studio_theme.dart';

/// Dialog that renders and displays a sprite animation GIF.
class SpritePreviewDialog extends ConsumerStatefulWidget {
  const SpritePreviewDialog({
    super.key,
    required this.spriteset,
    required this.sprite,
  });

  final String spriteset;
  final String sprite;

  static Future<void> show(BuildContext context, {
    required String spriteset,
    required String sprite,
  }) {
    return showDialog(
      context: context,
      builder: (_) => SpritePreviewDialog(spriteset: spriteset, sprite: sprite),
    );
  }

  @override
  ConsumerState<SpritePreviewDialog> createState() => _SpritePreviewDialogState();
}

class _SpritePreviewDialogState extends ConsumerState<SpritePreviewDialog> {
  Uint8List? _gifBytes;
  bool _loading = true;
  String? _error;

  @override
  void initState() {
    super.initState();
    _loadGif();
  }

  Future<void> _loadGif() async {
    setState(() {
      _loading = true;
      _error = null;
    });

    final resp = await ref.read(backendProvider.notifier).backend.renderSpriteGif(
      spriteset: widget.spriteset,
      sprite: widget.sprite,
    );

    if (!mounted) return;

    if (resp.containsKey('error')) {
      setState(() {
        _loading = false;
        _error = resp['error'] as String;
      });
      return;
    }

    final gif = resp['gif_b64'] as String?;
    if (gif != null) {
      setState(() {
        _gifBytes = base64Decode(gif);
        _loading = false;
      });
    } else {
      setState(() {
        _loading = false;
        _error = 'No GIF data in response';
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(8),
        side: StudioTheme.panelBorder,
      ),
      child: Container(
        width: 320,
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Row(
              children: [
                Icon(Icons.animation, size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Expanded(
                  child: Text(
                    '${widget.spriteset} / ${widget.sprite}',
                    style: theme.textTheme.bodyMedium!.copyWith(
                      fontSize: 14, fontWeight: FontWeight.w700,
                    ),
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                InkWell(
                  onTap: () => Navigator.of(context).pop(),
                  child: const Icon(Icons.close, size: 18),
                ),
              ],
            ),
            const SizedBox(height: 16),
            Container(
              constraints: const BoxConstraints(maxHeight: 256, maxWidth: 256),
              decoration: BoxDecoration(
                color: StudioTheme.canvasBg,
                borderRadius: BorderRadius.circular(4),
                border: Border.all(color: theme.dividerColor),
              ),
              child: _loading
                  ? const Center(
                      child: Padding(
                        padding: EdgeInsets.all(32),
                        child: CircularProgressIndicator(strokeWidth: 2),
                      ),
                    )
                  : _error != null
                      ? Padding(
                          padding: const EdgeInsets.all(16),
                          child: Text(_error!, style: theme.textTheme.bodySmall!.copyWith(
                            color: StudioTheme.error,
                          )),
                        )
                      : _gifBytes != null
                          ? Image.memory(
                              _gifBytes!,
                              filterQuality: FilterQuality.none,
                              fit: BoxFit.contain,
                            )
                          : const SizedBox.shrink(),
            ),
          ],
        ),
      ),
    );
  }
}
