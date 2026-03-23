import 'package:flutter/material.dart';

import '../theme/studio_theme.dart';

/// Keyboard shortcuts reference overlay, triggered by Cmd+/.
class ShortcutsDialog extends StatelessWidget {
  const ShortcutsDialog({super.key});

  static Future<void> show(BuildContext context) {
    return showDialog(
      context: context,
      builder: (_) => const ShortcutsDialog(),
    );
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
        width: 380,
        padding: const EdgeInsets.all(20),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(Icons.keyboard, size: 18, color: theme.colorScheme.primary),
                const SizedBox(width: 8),
                Text('Keyboard Shortcuts', style: theme.textTheme.bodyMedium!.copyWith(
                  fontSize: 16, fontWeight: FontWeight.w700,
                )),
                const Spacer(),
                InkWell(
                  onTap: () => Navigator.of(context).pop(),
                  child: const Icon(Icons.close, size: 18),
                ),
              ],
            ),
            const SizedBox(height: 16),
            _section('Tools'),
            _row('B', 'Pencil'),
            _row('E', 'Eraser'),
            _row('G', 'Bucket fill'),
            _row('I', 'Eyedropper'),
            const SizedBox(height: 12),
            _section('Canvas'),
            _row('Space + Drag', 'Pan canvas'),
            _row('Scroll', 'Zoom in/out'),
            _row('H', 'Toggle grid'),
            const SizedBox(height: 12),
            _section('Edit'),
            _row('\u2318Z', 'Undo'),
            _row('\u2318\u21E7Z', 'Redo'),
            _row('\u2318\u21E7V', 'Validate'),
            const SizedBox(height: 12),
            _section('Palette'),
            _row('Click', 'Select foreground'),
            _row('Shift + Click', 'Select background'),
            _row('Right-click', 'Select background'),
            const SizedBox(height: 12),
            _section('Navigation'),
            _row('\u2318/', 'This dialog'),
            _row('\u2318,', 'Settings'),
          ],
        ),
      ),
    );
  }

  Widget _section(String title) {
    return Builder(
      builder: (context) {
        final theme = Theme.of(context);
        return Padding(
          padding: const EdgeInsets.only(bottom: 6),
          child: Text(title, style: theme.textTheme.titleSmall),
        );
      },
    );
  }

  Widget _row(String shortcut, String action) {
    return Builder(
      builder: (context) {
        final theme = Theme.of(context);
        return Padding(
          padding: const EdgeInsets.only(bottom: 3),
          child: Row(
            children: [
              SizedBox(
                width: 100,
                child: Container(
                  padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: const Color(0xFF2a2a4e),
                    borderRadius: BorderRadius.circular(3),
                  ),
                  child: Text(
                    shortcut,
                    style: theme.textTheme.bodyMedium!.copyWith(fontSize: 11),
                  ),
                ),
              ),
              const SizedBox(width: 12),
              Text(action, style: theme.textTheme.bodySmall),
            ],
          ),
        );
      },
    );
  }
}
