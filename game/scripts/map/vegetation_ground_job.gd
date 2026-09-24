class_name VegetationGroundJob
extends RefCounted

## Lot C7b : recalage des arbres d'une tuile de végétation sur la surface de terrain affichée
## (relief fin 8192², LOD proche ou lointain), exécuté dans un `WorkerThreadPool`.
##
## Entrée : les tampons `MultiMesh` de la tuile (un par emplacement partie × essence, copie CPU
## gardée par `Vegetation`) et la grille du maillage affiché (`TerrainBuilder.surface_grid`,
## jamais modifiée après construction). Sortie : de nouveaux tampons, installés en une fois sur
## le fil principal (`MultiMesh.buffer`). Aucun accès à l'arbre de scène.

var tile_index: int = 0
## Génération de la tuile au lancement : une tuile libérée puis resemée entre-temps est ignorée.
var generation: int = 0
## Niveau de terrain (0 lointain, 1 proche, 2 fin) de la grille utilisée.
var level: int = 0
var origin: Vector2 = Vector2.ZERO
var grid: Dictionary = {}
var buffers: Array[PackedFloat32Array] = []
var results: Array[PackedFloat32Array] = []
var build_ms: float = 0.0


func run() -> void:
	var t0 := Time.get_ticks_usec()
	results.clear()
	for buffer in buffers:
		results.append(VegetationTileJob.reground(buffer, grid, origin))
	build_ms = (Time.get_ticks_usec() - t0) / 1000.0
