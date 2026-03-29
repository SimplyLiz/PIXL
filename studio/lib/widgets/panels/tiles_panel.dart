import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/pixel_canvas.dart';
import '../../providers/backend_provider.dart';
import '../../providers/canvas_provider.dart';
import '../../providers/tilemap_provider.dart';
import '../../theme/studio_theme.dart';

/// Left panel — vertical tile browser replacing the bottom VariantStrip.
///
/// Shows tile previews in a wrap/grid layout, with selection and
/// click-to-load-on-canvas behavior identical to the old VariantStrip.
class TilesPanel extends ConsumerStatefulWidget {
  const TilesPanel({super.key});

  @override
  ConsumerState<TilesPanel> createState() => _TilesPanelState();
}

class _TilesPanelState extends ConsumerState<TilesPanel> {
  String? _selectedTile;
  final Map<String, Uint8List> _previewCache = {};
  final Set<String> _loadingTiles = {};

  @override
  void initState() {
    super.initState();
    ref.listenManual(
      backendProvider.select((s) => s.tiles),
      (prev, tiles) => _loadMissingPreviews(tiles),
      fireImmediately: true,
    );
  }

  Future<void> _loadMissingPreviews(List<TileInfo> tiles) async {
    var didChange = false;
    for (final tile in tiles) {
      if (_previewCache.containsKey(tile.name)) continue;
      if (_loadingTiles.contains(tile.name)) continue;

      if (tile.previewBytes != null) {
        _previewCache[tile.name] = tile.previewBytes!;
        didChange = true;
        continue;
      }

      _loadingTiles.add(tile.name);
      final b64 = await ref.read(backendProvider.notifier).renderTile(
        tile.name,
        scale: 4,
      );
      _loadingTiles.remove(tile.name);
      if (b64 != null && mounted) {
        _previewCache[tile.name] = base64Decode(b64);
        didChange = true;
      }
    }
    if (didChange && mounted) setState(() {});
  }

  /// Guard against stale async results when tiles are clicked rapidly.
  String? _pendingTileLoad;

  Future<void> _loadTileToCanvas(String tileName) async {
    _pendingTileLoad = tileName;
    final result = await ref.read(backendProvider.notifier).getTilePixels(tileName);
    // Only apply if this is still the most recent click.
    if (_pendingTileLoad != tileName || result == null || !mounted) return;
    ref.read(canvasProvider.notifier).loadTilePixels(
      result.pixels,
      result.width,
      result.height,
    );
  }

  @override
  Widget build(BuildContext context) {
    final backend = ref.watch(backendProvider);
    final mode = ref.watch(editorModeProvider);
    final tilemapSelected = ref.watch(tilemapProvider.select((s) => s.selectedTile));
    final theme = Theme.of(context);

    return Container(
      width: 260,
      decoration: StudioTheme.panelDecoration,
      child: Column(
        children: [
          // Header
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 10, vertical: 7),
            decoration: const BoxDecoration(
              border: Border(bottom: StudioTheme.panelBorder),
            ),
            child: Row(
              children: [
                Icon(Icons.grid_view, size: 14, color: theme.colorScheme.primary),
                const SizedBox(width: 6),
                Text('TILES', style: theme.textTheme.titleSmall!.copyWith(fontSize: 10)),
                const Spacer(),
                Text(
                  '${backend.tiles.length}',
                  style: theme.textTheme.bodySmall!.copyWith(
                    fontSize: 10,
                    color: theme.colorScheme.primary,
                  ),
                ),
              ],
            ),
          ),

          // Tile grid
          Expanded(
            child: !backend.isConnected || backend.tiles.isEmpty
                ? Center(
                    child: Text(
                      backend.isConnected ? 'No tiles yet' : 'Engine offline',
                      style: theme.textTheme.bodySmall!.copyWith(
                        fontSize: 10,
                        color: theme.disabledColor,
                      ),
                    ),
                  )
                : GridView.builder(
                    padding: const EdgeInsets.all(6),
                    gridDelegate: const SliverGridDelegateWithFixedCrossAxisCount(
                      crossAxisCount: 3,
                      crossAxisSpacing: 6,
                      mainAxisSpacing: 6,
                      childAspectRatio: 0.85,
                    ),
                    itemCount: backend.tiles.length,
                    itemBuilder: (context, index) {
                      final tile = backend.tiles[index];
                      final isSelected = mode == EditorMode.tilemap
                          ? tile.name == tilemapSelected
                          : tile.name == _selectedTile;
                      final preview = _previewCache[tile.name];

                      return Tooltip(
                        message: '${tile.name}${tile.size != null ? ' (${tile.size})' : ''}',
                        child: InkWell(
                          onTap: () {
                            if (mode == EditorMode.tilemap) {
                              ref.read(tilemapProvider.notifier).setSelectedTile(tile.name);
                            } else {
                              setState(() => _selectedTile = tile.name);
                              _loadTileToCanvas(tile.name);
                            }
                          },
                          borderRadius: BorderRadius.circular(4),
                          child: Container(
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
                                    padding: const EdgeInsets.all(3),
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
                                    horizontal: 3, vertical: 2,
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
                      );
                    },
                  ),
          ),
        ],
      ),
    );
  }
}
