import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../theme/studio_theme.dart';

/// Which view is active in the left panel.
enum LeftPanelView { chat, tiles }

/// Provider for the active left panel view.
final leftPanelViewProvider = StateProvider<LeftPanelView>(
  (ref) => LeftPanelView.chat,
);

/// Thin icon strip on the far left to switch between Chat and Tiles views.
class LeftSidebar extends ConsumerWidget {
  const LeftSidebar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final activeView = ref.watch(leftPanelViewProvider);
    final theme = Theme.of(context);

    return Container(
      width: 36,
      decoration: BoxDecoration(
        color: theme.cardColor,
        border: const Border(right: StudioTheme.panelBorder),
      ),
      child: Column(
        children: [
          const SizedBox(height: 6),
          _SidebarIcon(
            icon: Icons.auto_awesome,
            tooltip: 'AI Chat',
            active: activeView == LeftPanelView.chat,
            onTap: () => ref.read(leftPanelViewProvider.notifier).state =
                LeftPanelView.chat,
          ),
          _SidebarIcon(
            icon: Icons.grid_view,
            tooltip: 'Tiles',
            active: activeView == LeftPanelView.tiles,
            onTap: () => ref.read(leftPanelViewProvider.notifier).state =
                LeftPanelView.tiles,
          ),
        ],
      ),
    );
  }
}

class _SidebarIcon extends StatelessWidget {
  const _SidebarIcon({
    required this.icon,
    required this.tooltip,
    required this.active,
    required this.onTap,
  });

  final IconData icon;
  final String tooltip;
  final bool active;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Tooltip(
      message: tooltip,
      preferBelow: false,
      child: InkWell(
        onTap: onTap,
        borderRadius: BorderRadius.circular(6),
        child: Container(
          width: 28,
          height: 28,
          margin: const EdgeInsets.symmetric(vertical: 2, horizontal: 4),
          decoration: BoxDecoration(
            color: active
                ? theme.colorScheme.primary.withValues(alpha: 0.2)
                : null,
            borderRadius: BorderRadius.circular(6),
            border: active
                ? Border.all(color: theme.colorScheme.primary, width: 1)
                : null,
          ),
          child: Icon(
            icon,
            size: 14,
            color: active
                ? theme.colorScheme.primary
                : theme.iconTheme.color,
          ),
        ),
      ),
    );
  }
}
