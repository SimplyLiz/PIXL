import 'package:flutter/material.dart';

/// Compact HSV color picker dialog with hue bar, SV square, RGB sliders, and hex input.
class ColorPickerDialog extends StatefulWidget {
  const ColorPickerDialog({super.key, required this.initialColor});

  final Color initialColor;

  static Future<Color?> show(BuildContext context, Color initialColor) {
    return showDialog<Color>(
      context: context,
      builder: (_) => ColorPickerDialog(initialColor: initialColor),
    );
  }

  @override
  State<ColorPickerDialog> createState() => _ColorPickerDialogState();
}

class _ColorPickerDialogState extends State<ColorPickerDialog> {
  late double _hue;
  late double _sat;
  late double _val;
  late TextEditingController _hexCtrl;

  @override
  void initState() {
    super.initState();
    final hsv = HSVColor.fromColor(widget.initialColor);
    _hue = hsv.hue;
    _sat = hsv.saturation;
    _val = hsv.value;
    _hexCtrl = TextEditingController(text: _hexString());
  }

  @override
  void dispose() {
    _hexCtrl.dispose();
    super.dispose();
  }

  Color get _color => HSVColor.fromAHSV(1.0, _hue, _sat, _val).toColor();

  int _r(Color c) => (c.r * 255).round();
  int _g(Color c) => (c.g * 255).round();
  int _b(Color c) => (c.b * 255).round();

  String _hexString() {
    final c = _color;
    return '${_r(c).toRadixString(16).padLeft(2, '0')}'
        '${_g(c).toRadixString(16).padLeft(2, '0')}'
        '${_b(c).toRadixString(16).padLeft(2, '0')}';
  }

  void _updateHex() {
    _hexCtrl.text = _hexString();
  }

  void _applyHex() {
    final hex = _hexCtrl.text.replaceAll('#', '').trim();
    final val = int.tryParse(hex, radix: 16);
    if (val != null && hex.length == 6) {
      final c = Color(0xFF000000 | val);
      final hsv = HSVColor.fromColor(c);
      setState(() {
        _hue = hsv.hue;
        _sat = hsv.saturation;
        _val = hsv.value;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final currentColor = _color;

    return Dialog(
      backgroundColor: theme.cardColor,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      child: Container(
        width: 280,
        padding: const EdgeInsets.all(16),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            // Header
            Row(
              children: [
                Text('Color Picker', style: theme.textTheme.bodyMedium!.copyWith(
                  fontWeight: FontWeight.w700,
                )),
                const Spacer(),
                InkWell(
                  onTap: () => Navigator.pop(context),
                  child: const Icon(Icons.close, size: 18),
                ),
              ],
            ),
            const SizedBox(height: 12),

            // SV square
            SizedBox(
              width: 248,
              height: 150,
              child: GestureDetector(
                onPanDown: (d) => _updateSV(d.localPosition, 248, 150),
                onPanUpdate: (d) => _updateSV(d.localPosition, 248, 150),
                child: CustomPaint(
                  painter: _SVPainter(hue: _hue, sat: _sat, val: _val),
                ),
              ),
            ),
            const SizedBox(height: 8),

            // Hue bar
            SizedBox(
              width: 248,
              height: 20,
              child: GestureDetector(
                onPanDown: (d) => _updateHueFromPos(d.localPosition.dx, 248),
                onPanUpdate: (d) => _updateHueFromPos(d.localPosition.dx, 248),
                child: CustomPaint(
                  painter: _HueBarPainter(hue: _hue),
                ),
              ),
            ),
            const SizedBox(height: 12),

            // Preview: old vs new
            Row(
              children: [
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('Old', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
                      Container(
                        height: 24,
                        decoration: BoxDecoration(
                          color: widget.initialColor,
                          borderRadius: BorderRadius.circular(3),
                          border: Border.all(color: theme.dividerColor),
                        ),
                      ),
                    ],
                  ),
                ),
                const SizedBox(width: 8),
                Expanded(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Text('New', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9)),
                      Container(
                        height: 24,
                        decoration: BoxDecoration(
                          color: currentColor,
                          borderRadius: BorderRadius.circular(3),
                          border: Border.all(color: theme.dividerColor),
                        ),
                      ),
                    ],
                  ),
                ),
              ],
            ),
            const SizedBox(height: 12),

            // RGB sliders
            _SliderRow('R', _r(currentColor), (v) {
              final c = Color.fromARGB(255, v, _g(currentColor), _b(currentColor));
              _setFromColor(c);
            }),
            _SliderRow('G', _g(currentColor), (v) {
              final c = Color.fromARGB(255, _r(currentColor), v, _b(currentColor));
              _setFromColor(c);
            }),
            _SliderRow('B', _b(currentColor), (v) {
              final c = Color.fromARGB(255, _r(currentColor), _g(currentColor), v);
              _setFromColor(c);
            }),
            const SizedBox(height: 8),

            // Hex input
            Row(
              children: [
                Text('#', style: theme.textTheme.bodySmall),
                const SizedBox(width: 4),
                SizedBox(
                  width: 80,
                  child: TextField(
                    controller: _hexCtrl,
                    style: theme.textTheme.bodyMedium!.copyWith(fontSize: 12),
                    decoration: InputDecoration(
                      isDense: true,
                      contentPadding: const EdgeInsets.symmetric(horizontal: 6, vertical: 4),
                      border: OutlineInputBorder(borderRadius: BorderRadius.circular(3)),
                    ),
                    onSubmitted: (_) => _applyHex(),
                  ),
                ),
                const Spacer(),
                ElevatedButton(
                  onPressed: () => Navigator.pop(context, currentColor),
                  style: ElevatedButton.styleFrom(
                    backgroundColor: theme.colorScheme.primary,
                    foregroundColor: Colors.white,
                    padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                    textStyle: const TextStyle(fontSize: 12),
                  ),
                  child: const Text('Apply'),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  void _updateSV(Offset pos, double w, double h) {
    setState(() {
      _sat = (pos.dx / w).clamp(0, 1);
      _val = 1.0 - (pos.dy / h).clamp(0, 1);
      _updateHex();
    });
  }

  void _updateHueFromPos(double x, double w) {
    setState(() {
      _hue = (x / w * 360).clamp(0, 360);
      _updateHex();
    });
  }

  void _setFromColor(Color c) {
    final hsv = HSVColor.fromColor(c);
    setState(() {
      _hue = hsv.hue;
      _sat = hsv.saturation;
      _val = hsv.value;
      _updateHex();
    });
  }
}

class _SliderRow extends StatelessWidget {
  const _SliderRow(this.label, this.value, this.onChanged);
  final String label;
  final int value;
  final ValueChanged<int> onChanged;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Row(
      children: [
        SizedBox(
          width: 14,
          child: Text(label, style: theme.textTheme.bodySmall!.copyWith(fontSize: 10)),
        ),
        Expanded(
          child: SliderTheme(
            data: SliderThemeData(
              trackHeight: 3,
              thumbShape: const RoundSliderThumbShape(enabledThumbRadius: 6),
              overlayShape: SliderComponentShape.noOverlay,
            ),
            child: Slider(
              value: value.toDouble(),
              min: 0,
              max: 255,
              onChanged: (v) => onChanged(v.round()),
            ),
          ),
        ),
        SizedBox(
          width: 28,
          child: Text('$value', style: theme.textTheme.bodySmall!.copyWith(fontSize: 9),
            textAlign: TextAlign.right),
        ),
      ],
    );
  }
}

// ── Custom Painters ──────────────────────────────────

class _SVPainter extends CustomPainter {
  _SVPainter({required this.hue, required this.sat, required this.val});
  final double hue, sat, val;

  @override
  void paint(Canvas canvas, Size size) {
    // Background: white to hue (horizontal), white to black (vertical)
    final hueColor = HSVColor.fromAHSV(1, hue, 1, 1).toColor();

    // White to hue gradient (horizontal)
    canvas.drawRect(
      Rect.fromLTWH(0, 0, size.width, size.height),
      Paint()..shader = LinearGradient(
        colors: [Colors.white, hueColor],
      ).createShader(Rect.fromLTWH(0, 0, size.width, size.height)),
    );

    // Transparent to black gradient (vertical)
    canvas.drawRect(
      Rect.fromLTWH(0, 0, size.width, size.height),
      Paint()..shader = const LinearGradient(
        begin: Alignment.topCenter,
        end: Alignment.bottomCenter,
        colors: [Colors.transparent, Colors.black],
      ).createShader(Rect.fromLTWH(0, 0, size.width, size.height)),
    );

    // Cursor
    final cx = sat * size.width;
    final cy = (1 - val) * size.height;
    canvas.drawCircle(Offset(cx, cy), 6, Paint()
      ..color = Colors.white
      ..style = PaintingStyle.stroke
      ..strokeWidth = 2);
    canvas.drawCircle(Offset(cx, cy), 5, Paint()
      ..color = Colors.black
      ..style = PaintingStyle.stroke
      ..strokeWidth = 1);
  }

  @override
  bool shouldRepaint(_SVPainter old) => hue != old.hue || sat != old.sat || val != old.val;
}

class _HueBarPainter extends CustomPainter {
  _HueBarPainter({required this.hue});
  final double hue;

  @override
  void paint(Canvas canvas, Size size) {
    final colors = List.generate(7, (i) =>
      HSVColor.fromAHSV(1, i * 60.0, 1, 1).toColor());

    canvas.drawRect(
      Rect.fromLTWH(0, 0, size.width, size.height),
      Paint()..shader = LinearGradient(colors: colors)
        .createShader(Rect.fromLTWH(0, 0, size.width, size.height)),
    );

    // Cursor
    final cx = hue / 360 * size.width;
    canvas.drawRect(
      Rect.fromLTWH(cx - 3, 0, 6, size.height),
      Paint()
        ..color = Colors.white
        ..style = PaintingStyle.stroke
        ..strokeWidth = 2,
    );
  }

  @override
  bool shouldRepaint(_HueBarPainter old) => hue != old.hue;
}
