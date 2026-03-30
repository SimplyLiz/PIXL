import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../providers/backend_provider.dart';
import '../providers/scanner_provider.dart';

/// Compact adapter picker for the top bar.
/// Shows the active adapter name and a dropdown to switch.
class AdapterPicker extends ConsumerStatefulWidget {
  const AdapterPicker({super.key});

  @override
  ConsumerState<AdapterPicker> createState() => _AdapterPickerState();
}

class _AdapterPickerState extends ConsumerState<AdapterPicker> {
  bool _loading = false;

  @override
  void initState() {
    super.initState();
    _refreshAdapters();
  }

  Future<void> _refreshAdapters() async {
    if (_loading) return;
    setState(() => _loading = true);

    final backend = ref.read(backendProvider.notifier).backend;
    final resp = await backend.listAdapters();

    if (resp.containsKey('adapters') && resp['adapters'] is List) {
      final list = (resp['adapters'] as List)
          .map((e) => AdapterInfo.fromJson(e as Map<String, dynamic>))
          .toList();
      ref.read(scannerProvider.notifier).setAdapters(list);
    }

    if (mounted) setState(() => _loading = false);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final scanner = ref.watch(scannerProvider);
    final adapters = scanner.adapters;

    if (adapters.isEmpty && !_loading) {
      return const SizedBox.shrink();
    }

    final activeAdapter = scanner.activeAdapter;
    final activeName = activeAdapter != null
        ? adapters
            .where((a) => a.path == activeAdapter)
            .map((a) => a.name)
            .firstOrNull
        : null;

    return PopupMenuButton<String?>(
      tooltip: 'Style adapter',
      onSelected: (path) async {
        final backend = ref.read(backendProvider.notifier).backend;
        if (path == null) {
          ref.read(scannerProvider.notifier).setActiveAdapter(null);
        } else {
          await backend.activateAdapter(path);
          ref.read(scannerProvider.notifier).setActiveAdapter(path);
        }
      },
      itemBuilder: (_) => [
        PopupMenuItem<String?>(
          value: null,
          child: Row(
            children: [
              Icon(Icons.block, size: 14,
                color: activeAdapter == null ? theme.colorScheme.primary : null),
              const SizedBox(width: 8),
              Text('Base model (no adapter)',
                style: TextStyle(
                  fontSize: 12,
                  fontWeight: activeAdapter == null ? FontWeight.w600 : null,
                )),
            ],
          ),
        ),
        const PopupMenuDivider(),
        ...adapters.map((a) => PopupMenuItem<String?>(
          value: a.path,
          child: Row(
            children: [
              Icon(Icons.auto_awesome, size: 14,
                color: a.path == activeAdapter ? theme.colorScheme.primary : null),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    Text(a.name, style: TextStyle(
                      fontSize: 12,
                      fontWeight: a.path == activeAdapter ? FontWeight.w600 : null,
                    )),
                    if (a.trainSamples != null || a.epochs != null)
                      Text(
                        [
                          if (a.trainSamples != null) '${a.trainSamples} samples',
                          if (a.epochs != null) '${a.epochs} epochs',
                        ].join(' · '),
                        style: TextStyle(
                          fontSize: 10,
                          color: theme.colorScheme.onSurface.withValues(alpha: 0.5),
                        ),
                      ),
                  ],
                ),
              ),
            ],
          ),
        )),
      ],
      child: Container(
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(6),
          border: Border.all(
            color: activeAdapter != null
                ? theme.colorScheme.primary.withValues(alpha: 0.5)
                : theme.colorScheme.onSurface.withValues(alpha: 0.2),
          ),
          color: activeAdapter != null
              ? theme.colorScheme.primary.withValues(alpha: 0.1)
              : null,
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              activeAdapter != null ? Icons.auto_awesome : Icons.layers,
              size: 14,
              color: activeAdapter != null ? theme.colorScheme.primary : null,
            ),
            const SizedBox(width: 4),
            Text(
              activeName ?? 'Base',
              style: TextStyle(
                fontSize: 11,
                fontWeight: activeAdapter != null ? FontWeight.w600 : null,
                color: activeAdapter != null ? theme.colorScheme.primary : null,
              ),
            ),
            const SizedBox(width: 2),
            Icon(Icons.arrow_drop_down, size: 14,
              color: theme.colorScheme.onSurface.withValues(alpha: 0.5)),
          ],
        ),
      ),
    );
  }
}
