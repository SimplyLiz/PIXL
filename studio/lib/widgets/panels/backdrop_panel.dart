import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../providers/backdrop_provider.dart';
import '../../theme/studio_theme.dart';

/// Right-panel content for backdrop editor mode: layers + zones.
class BackdropPanel extends ConsumerWidget {
  const BackdropPanel({super.key});

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(backdropEditorProvider);
    final theme = Theme.of(context);

    if (state.paxPath == null) {
      return Padding(
        padding: const EdgeInsets.all(16),
        child: Text(
          'Open a backdrop PAX file to edit layers and zones.',
          style: theme.textTheme.bodySmall!.copyWith(fontSize: 11, color: theme.dividerColor),
        ),
      );
    }

    return SingleChildScrollView(
      padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // ── File info ──
          Text(state.paxPath?.split('/').last ?? '', style: theme.textTheme.bodySmall!.copyWith(
            fontSize: 10, color: theme.dividerColor,
          )),
          const SizedBox(height: 12),

          // ── Layers ──
          _SectionHeader(title: 'LAYERS', icon: Icons.layers, theme: theme),
          const SizedBox(height: 6),
          if (state.layers.isEmpty)
            Text('No layers', style: theme.textTheme.bodySmall!.copyWith(fontSize: 10, color: theme.dividerColor))
          else
            ...state.layers.asMap().entries.map((e) => _LayerRow(
              index: e.key,
              layer: e.value,
              theme: theme,
            )),
          const SizedBox(height: 16),

          // ── Zones ──
          _SectionHeader(title: 'ANIMATION ZONES', icon: Icons.grid_view, theme: theme),
          const SizedBox(height: 6),
          ...state.zones.asMap().entries.map((e) => _ZoneRow(
            index: e.key,
            zone: e.value,
            isSelected: e.key == state.selectedZoneIndex,
            theme: theme,
          )),
          const SizedBox(height: 8),
          _AddZoneButton(theme: theme),
          const SizedBox(height: 16),

          // ── Selected zone properties ──
          if (state.selectedZoneIndex != null &&
              state.selectedZoneIndex! < state.zones.length) ...[
            _SectionHeader(title: 'ZONE PROPERTIES', icon: Icons.tune, theme: theme),
            const SizedBox(height: 6),
            _ZoneProperties(
              zone: state.zones[state.selectedZoneIndex!],
              index: state.selectedZoneIndex!,
              theme: theme,
            ),
          ],

          if (state.error != null) ...[
            const SizedBox(height: 12),
            Container(
              padding: const EdgeInsets.all(8),
              decoration: BoxDecoration(
                color: StudioTheme.error.withValues(alpha: 0.1),
                borderRadius: BorderRadius.circular(4),
              ),
              child: Text(state.error!, style: theme.textTheme.bodySmall!.copyWith(
                fontSize: 10, color: StudioTheme.error,
              )),
            ),
          ],
        ],
      ),
    );
  }
}

class _SectionHeader extends StatelessWidget {
  const _SectionHeader({required this.title, required this.icon, required this.theme});
  final String title;
  final IconData icon;
  final ThemeData theme;

  @override
  Widget build(BuildContext context) {
    return Row(
      children: [
        Icon(icon, size: 14, color: theme.colorScheme.primary),
        const SizedBox(width: 6),
        Text(title, style: theme.textTheme.titleSmall),
      ],
    );
  }
}

class _LayerRow extends ConsumerWidget {
  const _LayerRow({required this.index, required this.layer, required this.theme});
  final int index;
  final LayerState layer;
  final ThemeData theme;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return Container(
      margin: const EdgeInsets.only(bottom: 4),
      padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(4),
        border: Border.all(color: theme.dividerColor),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            children: [
              InkWell(
                onTap: () {
                  ref.read(backdropEditorProvider.notifier).updateLayer(
                    index,
                    layer.copyWith(visible: !layer.visible),
                  );
                },
                child: Icon(
                  layer.visible ? Icons.visibility : Icons.visibility_off,
                  size: 14,
                  color: layer.visible ? theme.colorScheme.primary : theme.dividerColor,
                ),
              ),
              const SizedBox(width: 6),
              Expanded(
                child: Text(layer.name, style: theme.textTheme.bodySmall!.copyWith(
                  fontSize: 11, fontWeight: FontWeight.w600,
                )),
              ),
              // Blend mode
              Text(layer.blend, style: theme.textTheme.bodySmall!.copyWith(
                fontSize: 9, color: theme.dividerColor,
              )),
            ],
          ),
          const SizedBox(height: 4),
          // Opacity slider
          Row(
            children: [
              Text('Opacity', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
              Expanded(
                child: Slider(
                  value: layer.opacity,
                  min: 0, max: 1,
                  onChanged: (v) {
                    ref.read(backdropEditorProvider.notifier).updateLayer(
                      index, layer.copyWith(opacity: v),
                    );
                  },
                ),
              ),
              Text('${(layer.opacity * 100).round()}%',
                  style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
            ],
          ),
          // Scroll factor
          Row(
            children: [
              Text('Parallax', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
              Expanded(
                child: Slider(
                  value: layer.scrollFactor,
                  min: 0, max: 1,
                  onChanged: (v) {
                    ref.read(backdropEditorProvider.notifier).updateLayer(
                      index, layer.copyWith(scrollFactor: v),
                    );
                  },
                ),
              ),
              Text(layer.scrollFactor.toStringAsFixed(2),
                  style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
            ],
          ),
        ],
      ),
    );
  }
}

class _ZoneRow extends ConsumerWidget {
  const _ZoneRow({
    required this.index,
    required this.zone,
    required this.isSelected,
    required this.theme,
  });
  final int index;
  final ZoneState zone;
  final bool isSelected;
  final ThemeData theme;

  static const _colors = [
    Color(0xFF4CAF50), Color(0xFF2196F3), Color(0xFFFF9800),
    Color(0xFF9C27B0), Color(0xFFF44336), Color(0xFF00BCD4),
    Color(0xFFFFEB3B), Color(0xFF795548),
  ];

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final color = _colors[index % _colors.length];
    return InkWell(
      onTap: () => ref.read(backdropEditorProvider.notifier).selectZone(index),
      borderRadius: BorderRadius.circular(4),
      child: Container(
        margin: const EdgeInsets.only(bottom: 3),
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(4),
          color: isSelected ? theme.colorScheme.primary.withValues(alpha: 0.15) : null,
          border: Border.all(
            color: isSelected ? theme.colorScheme.primary : theme.dividerColor,
          ),
        ),
        child: Row(
          children: [
            Container(width: 8, height: 8, decoration: BoxDecoration(
              color: color, borderRadius: BorderRadius.circular(2),
            )),
            const SizedBox(width: 6),
            Expanded(
              child: Text(zone.name, style: theme.textTheme.bodySmall!.copyWith(
                fontSize: 11, fontWeight: isSelected ? FontWeight.w600 : null,
              )),
            ),
            Container(
              padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
              decoration: BoxDecoration(
                color: theme.dividerColor.withValues(alpha: 0.3),
                borderRadius: BorderRadius.circular(3),
              ),
              child: Text(zone.behavior, style: theme.textTheme.bodySmall!.copyWith(fontSize: 8)),
            ),
            const SizedBox(width: 4),
            InkWell(
              onTap: () => ref.read(backdropEditorProvider.notifier).removeZone(index),
              child: Icon(Icons.close, size: 12, color: theme.dividerColor),
            ),
          ],
        ),
      ),
    );
  }
}

class _AddZoneButton extends ConsumerWidget {
  const _AddZoneButton({required this.theme});
  final ThemeData theme;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    return InkWell(
      onTap: () {
        final zones = ref.read(backdropEditorProvider).zones;
        ref.read(backdropEditorProvider.notifier).addZone(ZoneState(
          name: 'zone_${zones.length}',
          x: 0, y: 0, w: 32, h: 32,
          behavior: 'cycle',
        ));
      },
      borderRadius: BorderRadius.circular(4),
      child: Container(
        padding: const EdgeInsets.symmetric(vertical: 6),
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(4),
          border: Border.all(color: theme.dividerColor, style: BorderStyle.solid),
        ),
        child: Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Icon(Icons.add, size: 14, color: theme.dividerColor),
            const SizedBox(width: 4),
            Text('Add Zone', style: theme.textTheme.bodySmall!.copyWith(
              fontSize: 10, color: theme.dividerColor,
            )),
          ],
        ),
      ),
    );
  }
}

class _ZoneProperties extends ConsumerWidget {
  const _ZoneProperties({
    required this.zone,
    required this.index,
    required this.theme,
  });
  final ZoneState zone;
  final int index;
  final ThemeData theme;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final notifier = ref.read(backdropEditorProvider.notifier);
    final inputStyle = theme.textTheme.bodySmall!.copyWith(fontSize: 11);
    final inputDecor = InputDecoration(
      isDense: true,
      contentPadding: const EdgeInsets.symmetric(horizontal: 8, vertical: 6),
      border: OutlineInputBorder(borderRadius: BorderRadius.circular(4)),
    );

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Name
        TextField(
          controller: TextEditingController(text: zone.name),
          style: inputStyle,
          decoration: inputDecor.copyWith(labelText: 'Name'),
          onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(name: v)),
        ),
        const SizedBox(height: 8),

        // Rect
        Row(children: [
          Expanded(child: TextField(
            controller: TextEditingController(text: '${zone.x}'),
            style: inputStyle, decoration: inputDecor.copyWith(labelText: 'X'),
            keyboardType: TextInputType.number,
            onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(x: int.tryParse(v) ?? zone.x)),
          )),
          const SizedBox(width: 4),
          Expanded(child: TextField(
            controller: TextEditingController(text: '${zone.y}'),
            style: inputStyle, decoration: inputDecor.copyWith(labelText: 'Y'),
            keyboardType: TextInputType.number,
            onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(y: int.tryParse(v) ?? zone.y)),
          )),
          const SizedBox(width: 4),
          Expanded(child: TextField(
            controller: TextEditingController(text: '${zone.w}'),
            style: inputStyle, decoration: inputDecor.copyWith(labelText: 'W'),
            keyboardType: TextInputType.number,
            onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(w: int.tryParse(v) ?? zone.w)),
          )),
          const SizedBox(width: 4),
          Expanded(child: TextField(
            controller: TextEditingController(text: '${zone.h}'),
            style: inputStyle, decoration: inputDecor.copyWith(labelText: 'H'),
            keyboardType: TextInputType.number,
            onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(h: int.tryParse(v) ?? zone.h)),
          )),
        ]),
        const SizedBox(height: 8),

        // Behavior dropdown
        DropdownButtonFormField<String>(
          value: zoneBehaviors.contains(zone.behavior) ? zone.behavior : 'cycle',
          items: zoneBehaviors.map((b) => DropdownMenuItem(value: b, child: Text(b, style: inputStyle))).toList(),
          onChanged: (v) {
            if (v != null) notifier.updateZone(index, zone.copyWith(behavior: v));
          },
          decoration: inputDecor.copyWith(labelText: 'Behavior'),
          style: inputStyle,
          isDense: true,
        ),
        const SizedBox(height: 8),

        // Behavior-specific params
        if (['cycle', 'wave', 'flicker'].contains(zone.behavior))
          TextField(
            controller: TextEditingController(text: zone.params['cycle']?.toString() ?? ''),
            style: inputStyle,
            decoration: inputDecor.copyWith(labelText: 'Cycle name'),
            onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(
              params: {...zone.params, 'cycle': v},
            )),
          ),

        if (['hscroll_sine', 'vscroll_sine'].contains(zone.behavior)) ...[
          const SizedBox(height: 4),
          Row(children: [
            Expanded(child: TextField(
              controller: TextEditingController(text: zone.params['amplitude']?.toString() ?? '3'),
              style: inputStyle, decoration: inputDecor.copyWith(labelText: 'Amplitude'),
              keyboardType: TextInputType.number,
              onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(
                params: {...zone.params, 'amplitude': int.tryParse(v)},
              )),
            )),
            const SizedBox(width: 4),
            Expanded(child: TextField(
              controller: TextEditingController(text: zone.params['period']?.toString() ?? '16'),
              style: inputStyle, decoration: inputDecor.copyWith(labelText: 'Period'),
              keyboardType: TextInputType.number,
              onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(
                params: {...zone.params, 'period': int.tryParse(v)},
              )),
            )),
          ]),
        ],

        if (zone.behavior == 'mosaic') ...[
          const SizedBox(height: 4),
          Row(children: [
            Expanded(child: TextField(
              controller: TextEditingController(text: zone.params['size_x']?.toString() ?? '2'),
              style: inputStyle, decoration: inputDecor.copyWith(labelText: 'Size X'),
              keyboardType: TextInputType.number,
              onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(
                params: {...zone.params, 'size_x': int.tryParse(v)},
              )),
            )),
            const SizedBox(width: 4),
            Expanded(child: TextField(
              controller: TextEditingController(text: zone.params['size_y']?.toString() ?? '2'),
              style: inputStyle, decoration: inputDecor.copyWith(labelText: 'Size Y'),
              keyboardType: TextInputType.number,
              onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(
                params: {...zone.params, 'size_y': int.tryParse(v)},
              )),
            )),
          ]),
        ],

        if (zone.behavior == 'wave') ...[
          const SizedBox(height: 4),
          TextField(
            controller: TextEditingController(text: zone.params['phase_rows']?.toString() ?? '4'),
            style: inputStyle,
            decoration: inputDecor.copyWith(labelText: 'Phase rows'),
            keyboardType: TextInputType.number,
            onSubmitted: (v) => notifier.updateZone(index, zone.copyWith(
              params: {...zone.params, 'phase_rows': int.tryParse(v)},
            )),
          ),
        ],
      ],
    );
  }
}
