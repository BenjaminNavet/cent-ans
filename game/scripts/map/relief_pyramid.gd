class_name ReliefPyramid
extends RefCounted

## Pyramide de relief streamée (chantier ZG, ADR 0036) : lit `data/map/relief_pyramid.json` et
## localise les tuiles 512² 16 bits de `data/map/pyramid/E{k}/{col}_{row}.png` (hors git).
## Étage k : une tuile couvre `256 / 2^k` unités monde. E0 = `data/map/height/` (TerrainBuilder).
## Sans cache, `is_available()` est faux et la carte se comporte comme avant.

const TILE_PX := 512
const ROOT_TILE_UNITS := 256.0

var map_dir: String = ""
var max_level: int = 0


func load_manifest(dir: String) -> bool:
	map_dir = dir
	return false  # lot ZG2


func is_available() -> bool:
	return max_level > 0


func tile_units(level: int) -> float:
	return ROOT_TILE_UNITS / float(1 << level)


## Vrai si la tuile existe dans le manifeste et sur disque.
func has_tile(_level: int, _col: int, _row: int) -> bool:
	return false  # lot ZG2


func tile_path(level: int, col: int, row: int) -> String:
	return map_dir.path_join("pyramid/E%d/%d_%d.png" % [level, col, row])


## Étage le plus fin disponible au point carte (x, y) en unités monde.
func finest_level_at(_x: float, _y: float) -> int:
	return 0  # lot ZG2
