import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/tab_provider.dart';
import '../theme/studio_theme.dart';

/// Horizontal tab bar showing all open documents.
class DocumentTabBar extends ConsumerWidget {
  const DocumentTabBar({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final tabState = ref.watch(tabManagerProvider);
    final theme = Theme.of(context);

    if (tabState.tabOrder.length <= 1) return const SizedBox.shrink();

    return Container(
      height: 30,
      decoration: const BoxDecoration(
        border: Border(bottom: StudioTheme.panelBorder),
        color: StudioTheme.recessedBg,
      ),
      child: ReorderableListView.builder(
        scrollDirection: Axis.horizontal,
        buildDefaultDragHandles: false,
        proxyDecorator: (child, _, __) => Material(
          color: Colors.transparent,
          child: child,
        ),
        onReorder: (oldIndex, newIndex) {
          ref.read(tabManagerProvider.notifier).reorderTab(oldIndex, newIndex);
        },
        itemCount: tabState.tabOrder.length,
        itemBuilder: (context, index) {
          final tabId = tabState.tabOrder[index];
          final doc = tabState.documents[tabId];
          if (doc == null) return SizedBox.shrink(key: ValueKey(tabId));

          final isActive = tabId == tabState.activeTabId;

          return ReorderableDragStartListener(
            key: ValueKey(tabId),
            index: index,
            child: GestureDetector(
              onTap: () => ref.read(tabManagerProvider.notifier).switchTab(tabId),
              child: Container(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                decoration: BoxDecoration(
                  color: isActive ? theme.cardColor : null,
                  border: Border(
                    bottom: isActive
                        ? BorderSide(color: theme.colorScheme.primary, width: 2)
                        : BorderSide.none,
                    right: StudioTheme.panelBorder,
                  ),
                ),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    if (doc.isDirty)
                      Padding(
                        padding: const EdgeInsets.only(right: 4),
                        child: Container(
                          width: 6, height: 6,
                          decoration: BoxDecoration(
                            shape: BoxShape.circle,
                            color: theme.colorScheme.primary,
                          ),
                        ),
                      ),
                    Text(
                      doc.name,
                      style: theme.textTheme.bodySmall!.copyWith(
                        fontSize: 11,
                        color: isActive ? theme.colorScheme.primary : null,
                        fontWeight: isActive ? FontWeight.w600 : null,
                      ),
                    ),
                    const SizedBox(width: 6),
                    InkWell(
                      onTap: () => _closeTab(context, ref, tabId, doc.isDirty),
                      borderRadius: BorderRadius.circular(8),
                      child: Icon(
                        Icons.close,
                        size: 12,
                        color: isActive ? theme.textTheme.bodySmall?.color : theme.disabledColor,
                      ),
                    ),
                  ],
                ),
              ),
            ),
          );
        },
      ),
    );
  }

  void _closeTab(BuildContext context, WidgetRef ref, String tabId, bool isDirty) async {
    if (isDirty) {
      final shouldClose = await showDialog<bool>(
        context: context,
        builder: (ctx) => AlertDialog(
          title: const Text('Unsaved changes', style: TextStyle(fontSize: 14)),
          content: const Text('Close without saving?', style: TextStyle(fontSize: 12)),
          actions: [
            TextButton(
              onPressed: () => Navigator.pop(ctx, false),
              child: const Text('Cancel'),
            ),
            TextButton(
              onPressed: () => Navigator.pop(ctx, true),
              child: const Text('Close'),
            ),
          ],
        ),
      );
      if (shouldClose != true) return;
    }
    ref.read(tabManagerProvider.notifier).closeTab(tabId);
  }
}

