class_name ForestDetailProfile
extends Resource

## Lot SZ4b : réglages de la couche « forêt dense » (`ForestDetail`, `res://resources/forest_detail.tres`).
## Purement visuel.
##
## Les arbres de la carte (`Vegetation`) sont semés au pas `Vegetation.spacing` (1,35 unité) pour des
## arbres de ~1 km : couvert continu en vue stratégique. Quand `MapPropScale.tree_scale` les réduit à
## leur taille réelle, ce semis devient clairsemé. La couche dense ajoute, autour du point visé,
## des arbres semés au pas fin `Vegetation.spacing × full_scale` : la part affichée
## `full_scale² × (1/s² − 1)` (s = échelle des arbres) garde un couvert constant.

## Échelle des arbres à laquelle la couche dense est entière (pas fin = pas de la carte × ce
## rapport). Par défaut `MapPropScale.tree_ratio` : couvert de la carte retrouvé à taille réelle.
@export var full_scale: float = 0.035
## En deçà de cette part (arbres encore grands), la couche est éteinte.
@export var min_fraction: float = 0.003
## Côté d'une cellule (unités monde, sous-multiple de la tuile de 256) et parties par côté.
@export var cell_size: float = 16.0
@export var parts_side: int = 4
## Rayon autour du point visé = `radius_factor` × distance du rig, borné ; décroissance de la part
## affichée à partir de `fade_from` × ce rayon.
@export var radius_factor: float = 3.5
@export var radius_min: float = 6.0
@export var radius_max: float = 50.0
@export var fade_from: float = 0.55
## Budget d'instances affichées : au-delà, le rayon se resserre (gain lissé, ≥ `min_gain`).
@export var instance_budget: int = 220000
@export var min_gain: float = 0.35
## Paliers de la part semée (graines gardées) : une cellule est resemée au palier supérieur
## quand la part voulue dépasse la part semée ; marge `keep_margin`.
@export var keep_levels: PackedFloat32Array = PackedFloat32Array([0.015625, 0.0625, 0.25, 1.0])
@export var keep_margin: float = 1.25
## Maillage détaillé (et ombres) des parties à moins de `detail_factor` × la distance du rig.
@export var detail_factor: float = 1.2
## Couloirs laissés sans arbres de part et d'autre des fleuves fins et des routes drapées (lot ZG5b),
## en mètres au-delà de la demi-largeur : berges, et houppiers qui débordent (~15 m).
@export var river_clearance_m: float = 25.0
@export var road_clearance_m: float = 10.0
## Semis simultanés, recalages simultanés, cellules gardées en cache.
@export var max_jobs: int = 3
@export var max_ground_jobs: int = 1
@export var max_cells: int = 96
## Instances gardées (mémoire : 64 octets par instance côté processeur et autant côté GPU) :
## au-delà, les cellules hors champ les plus anciennes sont libérées.
@export var max_stored_instances: int = 600000

static var _default: ForestDetailProfile = null


## Part [0, 1] de la couche dense à afficher pour une échelle d'arbres `tree_scale`.
func fraction_for(tree_scale: float) -> float:
	var s := maxf(tree_scale, 1e-4)
	return clampf(full_scale * full_scale * (1.0 / (s * s) - 1.0), 0.0, 1.0)


## Plus petit palier de `keep_levels` qui couvre `need` (avec la marge), 1 au plus.
func keep_for(need: float) -> float:
	var wanted := minf(need * keep_margin, 1.0)
	for level in keep_levels:
		if level >= wanted:
			return level
	return 1.0


static func shared() -> ForestDetailProfile:
	if _default == null:
		var path := "res://resources/forest_detail.tres"
		if ResourceLoader.exists(path):
			_default = load(path) as ForestDetailProfile
		if _default == null:
			_default = ForestDetailProfile.new()
	return _default
