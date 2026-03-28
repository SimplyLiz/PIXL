"""Feature computation for MAP-Elites tilemap evaluation.

Computes behavior descriptors (features) from tile-name grids,
used as the feature dimensions in the MAP-Elites archive.
"""

from __future__ import annotations
from collections import deque


def compute_features(
    grid: list[list[str]],
    affordance_map: dict[str, str],
) -> dict[str, float]:
    """Compute behavior descriptors from a tile-name grid.

    Args:
        grid: 2D list of tile names (rows x cols).
        affordance_map: tile_name -> affordance string (e.g. "walkable", "obstacle").

    Returns:
        Dict with feature values: wall_ratio, walkable_ratio, room_count,
        largest_room, border_solidity, transition_count.
    """
    h = len(grid)
    w = len(grid[0]) if h > 0 else 0
    total = h * w
    if total == 0:
        return {
            "wall_ratio": 0.0,
            "walkable_ratio": 0.0,
            "room_count": 0,
            "largest_room": 0,
            "border_solidity": 0.0,
            "transition_count": 0,
        }

    # Count affordance types
    obstacles = 0
    walkables = 0
    for row in grid:
        for t in row:
            aff = _base_affordance(t, affordance_map)
            if aff == "obstacle":
                obstacles += 1
            elif aff == "walkable":
                walkables += 1

    # BFS room counting (connected walkable regions)
    visited: set[tuple[int, int]] = set()
    rooms = 0
    largest_room = 0
    for y in range(h):
        for x in range(w):
            if (x, y) not in visited and _base_affordance(grid[y][x], affordance_map) == "walkable":
                size = _bfs_flood(grid, x, y, affordance_map, visited)
                rooms += 1
                largest_room = max(largest_room, size)

    # Border solidity: fraction of border cells that are obstacles
    border_cells = 0
    border_obstacles = 0
    for x in range(w):
        for y in [0, h - 1]:
            border_cells += 1
            if _base_affordance(grid[y][x], affordance_map) == "obstacle":
                border_obstacles += 1
    for y in range(1, h - 1):
        for x in [0, w - 1]:
            border_cells += 1
            if _base_affordance(grid[y][x], affordance_map) == "obstacle":
                border_obstacles += 1

    # Transition count: cells where a neighbor has a different base tile name
    transitions = 0
    for y in range(h):
        for x in range(w):
            base = _strip_rotation(grid[y][x])
            for dx, dy in [(1, 0), (0, 1)]:
                nx, ny = x + dx, y + dy
                if 0 <= nx < w and 0 <= ny < h:
                    if _strip_rotation(grid[ny][nx]) != base:
                        transitions += 1

    return {
        "wall_ratio": obstacles / total,
        "walkable_ratio": walkables / total,
        "room_count": rooms,
        "largest_room": largest_room,
        "border_solidity": border_obstacles / border_cells if border_cells > 0 else 0.0,
        "transition_count": transitions,
    }


def _base_affordance(tile_name: str, affordance_map: dict[str, str]) -> str:
    """Look up affordance, stripping rotation suffixes if needed."""
    if tile_name in affordance_map:
        return affordance_map[tile_name]
    # Strip rotation suffixes: _90, _180, _270
    base = _strip_rotation(tile_name)
    return affordance_map.get(base, "")


def _strip_rotation(name: str) -> str:
    """Strip rotation suffixes (_90, _180, _270) from a tile name."""
    for suffix in ("_270", "_180", "_90"):
        if name.endswith(suffix):
            return name[: -len(suffix)]
    return name


def _bfs_flood(
    grid: list[list[str]],
    start_x: int,
    start_y: int,
    affordance_map: dict[str, str],
    visited: set[tuple[int, int]],
) -> int:
    """BFS flood fill from (start_x, start_y) over walkable cells. Returns region size."""
    h = len(grid)
    w = len(grid[0])
    queue: deque[tuple[int, int]] = deque()
    queue.append((start_x, start_y))
    visited.add((start_x, start_y))
    size = 0
    while queue:
        x, y = queue.popleft()
        size += 1
        for dx, dy in [(1, 0), (-1, 0), (0, 1), (0, -1)]:
            nx, ny = x + dx, y + dy
            if 0 <= nx < w and 0 <= ny < h and (nx, ny) not in visited:
                if _base_affordance(grid[ny][nx], affordance_map) == "walkable":
                    visited.add((nx, ny))
                    queue.append((nx, ny))
    return size


def path_connectivity(grid: list[list[str]], affordance_map: dict[str, str]) -> float:
    """Fraction of walkable cells reachable from the largest walkable region."""
    h = len(grid)
    w = len(grid[0]) if h > 0 else 0
    total_walkable = sum(
        1 for row in grid for t in row if _base_affordance(t, affordance_map) == "walkable"
    )
    if total_walkable == 0:
        return 0.0

    # Find largest room
    visited: set[tuple[int, int]] = set()
    largest = 0
    for y in range(h):
        for x in range(w):
            if (x, y) not in visited and _base_affordance(grid[y][x], affordance_map) == "walkable":
                size = _bfs_flood(grid, x, y, affordance_map, visited)
                largest = max(largest, size)

    return largest / total_walkable


# --- Self-test ---
if __name__ == "__main__":
    # Quick sanity check
    test_grid = [
        ["wall", "wall", "wall", "wall"],
        ["wall", "floor", "floor", "wall"],
        ["wall", "floor", "floor", "wall"],
        ["wall", "wall", "wall", "wall"],
    ]
    test_aff = {"wall": "obstacle", "floor": "walkable"}
    features = compute_features(test_grid, test_aff)
    print(f"Features: {features}")
    assert features["wall_ratio"] == 0.75, f"wall_ratio: {features['wall_ratio']}"
    assert features["walkable_ratio"] == 0.25
    assert features["room_count"] == 1
    assert features["largest_room"] == 4
    assert features["border_solidity"] == 1.0
    conn = path_connectivity(test_grid, test_aff)
    print(f"Path connectivity: {conn}")
    assert conn == 1.0
    print("All checks passed.")
