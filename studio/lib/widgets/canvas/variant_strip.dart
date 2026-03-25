import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/pixel_canvas.dart';
import '../../providers/backend_provider.dart';
import '../../providers/tilemap_provider.dart';
import '../../theme/studio_theme.dart';

/// Horizontal strip below the canvas showing tile previews from the session.
/// Click a tile to select it in the tile list and show its render.
class VariantStrip extends ConsumerStatefulWidget {
  const VariantStrip({super.key});

  @override
  ConsumerState<VariantStrip> createState() => _VariantStripState();
}

class _VariantStripState extends ConsumerState<VariantStrip> {
  String? _selectedTile;
  final Map<String, Uint8List> _previewCache = {};
  final Set<String> _loadingTiles = {};

  @override
  void initState() {
    super.initState();
    // Listen for tile list changes and load missing previews.
    ref.listenManual(
      backendProvider.select((s) => s.tiles),
      (_, tiles) => _loadMissingPreviews(tiles),
    );
  }

  Future<void> _loadMissingPreviews(List<TileInfo> tiles) async {
    for (final tile in tiles) {
      if (_previewCache.containsKey(tile.name)) continue;
      if (_loadingTiles.contains(tile.name)) continue;

      if (tile.previewBytes != null) {
        _previewCache[tile.name] = tile.previewBytes!;
        if (mounted) setState(() {});
        continue;
      }

      _loadingTiles.add(tile.name);
      final b64 = await ref.read(backendProvider.notifier).renderTile(
        tile.name,
        scale: 4,
      );
      _loadingTiles.remove(tile.name);
      if (b64 != null && mounted) {
        setState(() {
          _previewCache[tile.name] = base64Decode(b64);
        });
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final backend = ref.watch(backendProvider);
    final mode = ref.watch(editorModeProvider);
    final tilemapSelected = ref.watch(tilemapProvider.select((s) => s.selectedTile));
    final theme = Theme.of(context);

    if (!backend.isConnected || backend.tiles.isEmpty) {
      return const SizedBox.shrink();
    }

    return Container(
      height: 72,
      decoration: const BoxDecoration(
        color: StudioTheme.canvasBg,
        border: Border(top: StudioTheme.panelBorder),
      ),
      child: Row(
        children: [
          // Label
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: RotatedBox(
              quarterTurns: -1,
              child: Text(
                'TILES',
                style: theme.textTheme.titleSmall!.copyWith(fontSize: 9),
              ),
            ),
          ),
          Container(width: 1, color: theme.dividerColor),

          // Scrollable tile strip
          Expanded(
            child: ListView.builder(
              scrollDirection: Axis.horizontal,
              padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 6),
              itemCount: backend.tiles.length,
              itemBuilder: (context, index) {
                final tile = backend.tiles[index];
                final isSelected = mode == EditorMode.tilemap
                    ? tile.name == tilemapSelected
                    : tile.name == _selectedTile;
                final preview = _previewCache[tile.name];

                return Padding(
                  padding: const EdgeInsets.only(right: 6),
                  child: Tooltip(
                    message: '${tile.name}${tile.size != null ? ' (${tile.size})' : ''}',
                    child: InkWell(
                      onTap: () {
                        if (mode == EditorMode.tilemap) {
                          ref.read(tilemapProvider.notifier).setSelectedTile(tile.name);
                        } else {
                          setState(() => _selectedTile = tile.name);
                        }
                      },
                      borderRadius: BorderRadius.circular(4),
                      child: Container(
                        width: 56,
                        decoration: BoxDecoration(
                          color: StudioTheme.recessedBg,
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(
                            color: isSelected
                                ? theme.colorScheme.primary
                                : theme.dividerColor,
                            width: isSelected ? 2 : 1,
                          ),
                        ),
                        child: Column(
                          children: [
                            Expanded(
                              child: Padding(
                                padding: const EdgeInsets.all(2),
                                child: preview != null
                                    ? Image.memory(
                                        preview,
                                        filterQuality: FilterQuality.none,
                                        fit: BoxFit.contain,
                                      )
                                    : const Center(
                                        child: SizedBox(
                                          width: 12, height: 12,
                                          child: CircularProgressIndicator(
                                            strokeWidth: 1,
                                          ),
                                        ),
                                      ),
                              ),
                            ),
                            Container(
                              width: double.infinity,
                              padding: const EdgeInsets.symmetric(
                                horizontal: 2, vertical: 1,
                              ),
                              decoration: BoxDecoration(
                                color: isSelected
                                    ? theme.colorScheme.primary.withValues(alpha: 0.2)
                                    : StudioTheme.canvasBg,
                                borderRadius: const BorderRadius.only(
                                  bottomLeft: Radius.circular(3),
                                  bottomRight: Radius.circular(3),
                                ),
                              ),
                              child: Text(
                                tile.name,
                                style: TextStyle(
                                  fontSize: 7,
                                  color: isSelected
                                      ? theme.colorScheme.primary
                                      : theme.textTheme.bodySmall?.color,
                                ),
                                overflow: TextOverflow.ellipsis,
                                textAlign: TextAlign.center,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                );
              },
            ),
          ),

          // Tile count badge
          Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8),
            child: Text(
              '${backend.tiles.length}',
              style: theme.textTheme.bodySmall!.copyWith(
                fontSize: 10,
                color: theme.colorScheme.primary,
              ),
            ),
          ),
        ],
      ),
    );
  }
}
