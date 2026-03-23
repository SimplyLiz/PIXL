import 'package:flutter_test/flutter_test.dart';
import 'package:pixl_studio/utils/grid_parser.dart';

void main() {
  group('extractGrid', () {
    test('extracts grid from code block', () {
      const response = '''
Here's your tile:
```
####
#..#
#..#
####
```
Hope you like it!
''';
      final grid = extractGrid(response);
      expect(grid, isNotNull);
      expect(grid, contains('####'));
      expect(grid!.split('\n').length, 4);
    });

    test('extracts grid from labeled code block', () {
      const response = '''
```pax
####
#+.#
#.+#
####
```
''';
      final grid = extractGrid(response);
      expect(grid, isNotNull);
      expect(grid!.split('\n').length, 4);
    });

    test('extracts raw grid lines without code fences', () {
      const response = 'Here is the tile:\n\n####\n#..#\n#..#\n####\n\nThat is your wall tile.';
      final grid = extractGrid(response);
      expect(grid, isNotNull);
      expect(grid, '####\n#..#\n#..#\n####');
    });

    test('returns null for no grid', () {
      const response = 'Sorry, I cannot generate that tile.';
      expect(extractGrid(response), isNull);
    });

    test('returns null for too-short grid', () {
      const response = '''
```
##
##
```
''';
      expect(extractGrid(response), isNull);
    });

    test('ignores markdown headers and lists', () {
      const response = '''
# Tile Design
- Use dark colors
- Add texture

```
########
#++++++#
#+....+#
#+....+#
#+....+#
#+....+#
#++++++#
########
```
''';
      final grid = extractGrid(response);
      expect(grid, isNotNull);
      expect(grid!.split('\n').length, 8);
    });
  });

  group('generateTileName', () {
    test('extracts meaningful words', () {
      final name = generateTileName('generate a dungeon wall tile');
      expect(name, startsWith('dungeon_wall_'));
    });

    test('handles empty prompt', () {
      final name = generateTileName('generate');
      expect(name, startsWith('tile_'));
    });

    test('strips special characters', () {
      final name = generateTileName('create a "mossy" stone!');
      expect(name, contains('mossy'));
      expect(name, contains('stone'));
    });

    test('limits to 3 words', () {
      final name = generateTileName('make a dark mossy stone dungeon wall floor');
      final prefix = name.split('_').take(3).join('_');
      // Should have at most 3 content words before the timestamp suffix
      expect(prefix.split('_').length, 3);
    });
  });
}
