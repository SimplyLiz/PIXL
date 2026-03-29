import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../models/animation_state.dart';
import '../../providers/animation_provider.dart';
import '../../providers/canvas_provider.dart';
import '../../theme/studio_theme.dart';

/// Animation bar at the bottom of the canvas area.
///
/// - When no animation: shows a single "+" button to add a frame.
/// - When animation active: shows frame thumbnails, playback controls,
///   add/remove frame buttons, and FPS control.
///
/// Uses [AnimationController] with vsync for smooth, frame-synced playback
/// instead of [Timer.periodic].
class AnimationBar extends ConsumerStatefulWidget {
  const AnimationBar({super.key});

  @override
  ConsumerState<AnimationBar> createState() => _AnimationBarState();
}

class _AnimationBarState extends ConsumerState<AnimationBar>
    with SingleTickerProviderStateMixin {
  late final AnimationController _playbackController;
  final _scrollController = ScrollController();

  @override
  void initState() {
    super.initState();
    _playbackController = AnimationController(
      vsync: this,
      duration: const Duration(seconds: 1), // recalculated on play
    );
  }

  @override
  void dispose() {
    _playbackController.dispose();
    _scrollController.dispose();
    super.dispose();
  }

  void _loadFrame(AnimationFrame frame) {
    ref.read(canvasProvider.notifier).loadFramePixels(frame.layerPixels);
  }

  void _startAnimation() {
    final canvas = ref.read(canvasProvider);
    ref.read(spriteAnimationProvider.notifier).startAnimation(canvas);
    _scrollToEnd();
  }

  void _addFrame({bool duplicate = false}) {
    final canvas = ref.read(canvasProvider);
    ref.read(spriteAnimationProvider.notifier)
        .addFrame(canvas, duplicate: duplicate);
    final anim = ref.read(spriteAnimationProvider);
    if (anim.currentFrame < anim.frames.length) {
      _loadFrame(anim.frames[anim.currentFrame]);
    }
    _scrollToEnd();
  }

  void _removeFrame(int index) {
    final canvas = ref.read(canvasProvider);
    final frame = ref.read(spriteAnimationProvider.notifier)
        .removeFrame(index, canvas);
    if (frame != null) {
      _loadFrame(frame);
    }
  }

  void _switchFrame(int index) {
    final canvas = ref.read(canvasProvider);
    final frame = ref.read(spriteAnimationProvider.notifier)
        .switchFrame(index, canvas);
    if (frame != null) {
      _loadFrame(frame);
    }
  }

  void _scrollToEnd() {
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (_scrollController.hasClients) {
        _scrollController.animateTo(
          _scrollController.position.maxScrollExtent,
          duration: const Duration(milliseconds: 150),
          curve: Curves.easeOut,
        );
      }
    });
  }

  void _startPlayback(SpriteAnimationState anim) {
    ref.read(spriteAnimationProvider.notifier).setPlaying(true);
    final frameDuration = Duration(
      milliseconds: (1000 / anim.fps.clamp(1, 60)).round(),
    );
    _playbackController
      ..stop()
      ..duration = frameDuration
      ..forward();

    _playbackController.addStatusListener(_onPlaybackTick);
  }

  void _onPlaybackTick(AnimationStatus status) {
    if (status != AnimationStatus.completed) return;

    final state = ref.read(spriteAnimationProvider);
    if (!state.isPlaying || state.frames.length <= 1) {
      _stopPlayback();
      return;
    }

    final canvas = ref.read(canvasProvider);
    final frame = ref.read(spriteAnimationProvider.notifier)
        .advanceFrame(canvas);
    if (frame != null) {
      _loadFrame(frame);
    }

    // Restart controller for next frame.
    _playbackController
      ..reset()
      ..forward();
  }

  void _stopPlayback() {
    _playbackController
      ..removeStatusListener(_onPlaybackTick)
      ..stop()
      ..reset();
    ref.read(spriteAnimationProvider.notifier).setPlaying(false);
  }

  @override
  Widget build(BuildContext context) {
    final anim = ref.watch(spriteAnimationProvider);
    final theme = Theme.of(context);

    // No animation mode — show minimal bar with "add frame" entry point.
    if (anim.frames.isEmpty) {
      return Container(
        height: 32,
        decoration: const BoxDecoration(
          color: StudioTheme.canvasBg,
          border: Border(top: StudioTheme.panelBorder),
        ),
        padding: const EdgeInsets.symmetric(horizontal: 10),
        child: Row(
          children: [
            RotatedBox(
              quarterTurns: -1,
              child: Text(
                'ANIM',
                style: theme.textTheme.titleSmall!.copyWith(fontSize: 8),
              ),
            ),
            const SizedBox(width: 8),
            Text(
              '1/1',
              style: TextStyle(fontSize: 10, color: theme.disabledColor),
            ),
            const Spacer(),
            Tooltip(
              message: 'Add frame to start animating',
              child: InkWell(
                onTap: _startAnimation,
                borderRadius: BorderRadius.circular(4),
                child: Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
                  decoration: BoxDecoration(
                    borderRadius: BorderRadius.circular(4),
                    border: Border.all(color: theme.dividerColor),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      Icon(Icons.add,
                          size: 12, color: theme.colorScheme.primary),
                      const SizedBox(width: 4),
                      Text(
                        'Add Frame',
                        style: TextStyle(
                          fontSize: 9,
                          color: theme.colorScheme.primary,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
          ],
        ),
      );
    }

    // Animation mode active — full controls.
    return Container(
      decoration: const BoxDecoration(
        color: StudioTheme.canvasBg,
        border: Border(top: StudioTheme.panelBorder),
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          // Frame strip
          SizedBox(
            height: 34,
            child: Row(
              children: [
                // Label
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 6),
                  child: RotatedBox(
                    quarterTurns: -1,
                    child: Text(
                      'ANIM',
                      style: theme.textTheme.titleSmall!.copyWith(fontSize: 8),
                    ),
                  ),
                ),
                Container(width: 1, color: theme.dividerColor),

                // Frame thumbnails
                Expanded(
                  child: ListView.builder(
                    controller: _scrollController,
                    scrollDirection: Axis.horizontal,
                    padding:
                        const EdgeInsets.symmetric(horizontal: 4, vertical: 2),
                    itemCount: anim.frames.length,
                    itemBuilder: (context, index) {
                      final isActive = index == anim.currentFrame;
                      // Active frame pixels live on canvas; others in frames list.
                      final canvas = ref.watch(canvasProvider);
                      final pixels = isActive
                          ? [for (final l in canvas.layers) l.pixels]
                          : anim.frames[index].layerPixels;
                      return _FrameThumb(
                        key: ValueKey('frame_$index'),
                        index: index,
                        isActive: isActive,
                        isPlaying: anim.isPlaying,
                        layerPixels: pixels,
                        canvasWidth: canvas.width,
                        canvasHeight: canvas.height,
                        onTap: () => _switchFrame(index),
                        onContextMenu: (pos) =>
                            _showFrameMenu(pos, index),
                      );
                    },
                  ),
                ),

                // Add frame buttons
                Padding(
                  padding: const EdgeInsets.only(right: 4),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      _BarIconBtn(
                        icon: Icons.add,
                        tooltip: 'Add blank frame',
                        onTap:
                            anim.isPlaying ? null : () => _addFrame(),
                      ),
                      _BarIconBtn(
                        icon: Icons.copy,
                        tooltip: 'Duplicate frame',
                        onTap: anim.isPlaying
                            ? null
                            : () => _addFrame(duplicate: true),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),

          // Playback controls
          SizedBox(
            height: 22,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8),
              child: Row(
              children: [
                // Play/Stop
                _BarIconBtn(
                  icon: anim.isPlaying ? Icons.stop : Icons.play_arrow,
                  tooltip: anim.isPlaying ? 'Stop' : 'Play',
                  onTap: () {
                    if (anim.isPlaying) {
                      _stopPlayback();
                    } else {
                      _startPlayback(anim);
                    }
                  },
                ),
                const SizedBox(width: 4),

                // Frame counter
                Text(
                  '${anim.currentFrame + 1}/${anim.totalFrames}',
                  style: TextStyle(
                    fontSize: 10,
                    color: theme.textTheme.bodySmall?.color,
                  ),
                ),

                const Spacer(),

                // FPS control
                _BarIconBtn(
                  icon: Icons.remove,
                  tooltip: 'Decrease FPS',
                  onTap: anim.fps > 1
                      ? () => ref
                          .read(spriteAnimationProvider.notifier)
                          .setFps(anim.fps - 1)
                      : null,
                ),
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 2),
                  child: Container(
                    padding:
                        const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(3),
                      border: Border.all(color: theme.dividerColor),
                    ),
                    child: Text(
                      '${anim.fps} fps',
                      style: TextStyle(
                        fontSize: 8,
                        color: theme.textTheme.bodySmall?.color,
                      ),
                    ),
                  ),
                ),
                _BarIconBtn(
                  icon: Icons.add,
                  tooltip: 'Increase FPS',
                  onTap: anim.fps < 30
                      ? () => ref
                          .read(spriteAnimationProvider.notifier)
                          .setFps(anim.fps + 1)
                      : null,
                ),
              ],
              ),
            ),
          ),
        ],
      ),
    );
  }

  void _showFrameMenu(Offset globalPosition, int index) {
    final overlay = Overlay.of(context).context.findRenderObject() as RenderBox;
    final position = RelativeRect.fromRect(
      globalPosition & const Size(1, 1),
      Offset.zero & overlay.size,
    );
    final theme = Theme.of(context);
    showMenu<String>(
      context: context,
      position: position,
      color: theme.cardColor,
      items: [
        const PopupMenuItem(
          value: 'duplicate',
          child: Text('Duplicate', style: TextStyle(fontSize: 12)),
        ),
        const PopupMenuItem(
          value: 'delete',
          child: Text('Delete', style: TextStyle(fontSize: 12)),
        ),
      ],
    ).then((value) {
      if (value == 'duplicate') {
        _switchFrame(index);
        _addFrame(duplicate: true);
      } else if (value == 'delete') {
        _removeFrame(index);
      }
    });
  }
}

// ── Frame thumbnail with pixel preview ──────────────────────

class _FrameThumb extends StatelessWidget {
  const _FrameThumb({
    super.key,
    required this.index,
    required this.isActive,
    required this.isPlaying,
    required this.layerPixels,
    required this.canvasWidth,
    required this.canvasHeight,
    required this.onTap,
    required this.onContextMenu,
  });

  final int index;
  final bool isActive;
  final bool isPlaying;
  final List<List<Color?>> layerPixels;
  final int canvasWidth;
  final int canvasHeight;
  final VoidCallback onTap;
  final ValueChanged<Offset> onContextMenu;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Padding(
      padding: const EdgeInsets.only(right: 3),
      child: GestureDetector(
        onTap: isPlaying ? null : onTap,
        onSecondaryTapUp: isPlaying
            ? null
            : (details) => onContextMenu(details.globalPosition),
        onLongPressStart: isPlaying
            ? null
            : (details) => onContextMenu(details.globalPosition),
        child: Container(
          width: 30,
          decoration: BoxDecoration(
            color: StudioTheme.recessedBg,
            borderRadius: BorderRadius.circular(3),
            border: Border.all(
              color: isActive ? theme.colorScheme.primary : theme.dividerColor,
              width: isActive ? 2 : 1,
            ),
          ),
          child: ClipRRect(
            borderRadius: BorderRadius.circular(2),
            child: CustomPaint(
              painter: _FramePreviewPainter(
                layerPixels: layerPixels,
                canvasWidth: canvasWidth,
                canvasHeight: canvasHeight,
              ),
            ),
          ),
        ),
      ),
    );
  }
}

/// Paints a tiny pixel preview of a single animation frame.
class _FramePreviewPainter extends CustomPainter {
  _FramePreviewPainter({
    required this.layerPixels,
    required this.canvasWidth,
    required this.canvasHeight,
  });

  final List<List<Color?>> layerPixels;
  final int canvasWidth;
  final int canvasHeight;

  @override
  void paint(Canvas canvas, Size size) {
    if (canvasWidth == 0 || canvasHeight == 0) return;

    final ps = size.width / canvasWidth;
    final paint = Paint();

    // Composite all layers bottom-up.
    for (final pixels in layerPixels) {
      for (var y = 0; y < canvasHeight; y++) {
        for (var x = 0; x < canvasWidth; x++) {
          final i = y * canvasWidth + x;
          if (i >= pixels.length) continue;
          final color = pixels[i];
          if (color == null) continue;
          paint.color = color;
          canvas.drawRect(
            Rect.fromLTWH(x * ps, y * ps, ps, ps),
            paint,
          );
        }
      }
    }
  }

  @override
  bool shouldRepaint(_FramePreviewPainter old) => true;
}

// ── Small icon button used in the bar ───────────────────────

class _BarIconBtn extends StatelessWidget {
  const _BarIconBtn({
    required this.icon,
    required this.tooltip,
    required this.onTap,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Tooltip(
      message: tooltip,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(3),
        child: Padding(
          padding: const EdgeInsets.all(2),
          child: Icon(
            icon,
            size: 14,
            color: onTap != null ? theme.iconTheme.color : theme.disabledColor,
          ),
        ),
      ),
    );
  }
}
