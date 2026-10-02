class_name ForestDetailProfile
extends Resource

## Lot SZ4b : réglages de la couche « forêt dense » (`ForestDetail`, `res://resources/forest_detail.tres`).
## Purement visuel.
##
## Les arbres de la carte (`Vegetation`) sont semés au pas `Vegetation.spacing` (1,35 unité, ~970 m) :
## à l'échelle 1:1 (VT3, `MapPropScale.tree_scale()`), ce semis n'est qu'un arbre isolé par km². La
## couche dense ajoute, autour du point visé, des arbres semés au pas fin
## `Vegetation.spacing × full_scale` qui ferment le couvert des forêts, en deçà de la portée des
## arbres (`MapPropScale.tree_max_distance`) ; au-delà, la canopée du terrain porte la forêt.

## Pas fin = pas de la carte × ce rapport. VT3 : 0,022 → ~21 m entre deux arbres pour des houppiers
## de 14-28 m (arbres à `tree_ratio` 0,018) : couvert fermé au cœur des massifs. Avant VT3, le pas
## demandé (0,047 u) était relevé au plancher natif de 0,05 u (36 m) ; plancher abaissé à 0,01.
@export var full_scale: float = 0.022
## VT3 : part affichée selon la distance du rig : 1 jusqu'à `dense_full_distance`, puis
## `far_share` à la portée des arbres (arbres de 1-2 px sur la canopée du terrain : un semis plus
## clair suffit à donner le grain, et le passage à la canopée se fait en douceur).
@export var dense_full_distance: float = 8.0
@export var far_share: float = 0.35
## En deçà de cette part (arbres encore grands), la couche est éteinte.
@export var min_fraction: float = 0.003
## Côté d'une cellule (unités monde, sous-multiple de la tuile de 256) et parties par côté.
## VT3 : 8 u × 2 parties (parties de 4 u comme avant) : ~3 × plus d'arbres par unité², une cellule
## pleine reste sous ~75 000 instances.
@export var cell_size: float = 8.0
@export var parts_side: int = 2
## Rayon autour du point visé = `radius_factor` × distance du rig, borné (et par la portée
## `MapPropScale.tree_view_range`), × le gain du budget ; décroissance de la part affichée à partir
## de `fade_from` × ce rayon.
@export var radius_factor: float = 3.5
@export var radius_min: float = 6.0
@export var radius_max: float = 50.0
@export var fade_from: float = 0.4
## Budget d'instances affichées : au-delà, le rayon se resserre (gain lissé, ≥ `min_gain`).
@export var instance_budget: int = 300000
@export var min_gain: float = 0.35
## Paliers de la part semée (graines gardées) : une cellule est resemée au palier supérieur
## quand la part voulue dépasse la part semée ; marge `keep_margin`.
@export var keep_levels: PackedFloat32Array = PackedFloat32Array([0.016025, 0.0625, 0.25, 1.0])
@export var keep_margin: float = 1.25
## Ombres des parties à moins de `detail_factor` × la distance du rig (maillage bas partout).
@export var detail_factor: float = 1.2
## Couloirs laissés sans arbres de part et d'autre des fleuves fins et des routes drapées (lot ZG5b),
## en mètres au-delà de la demi-largeur : berges, et houppiers qui débordent (~15 m).
@export var river_clearance_m: float = 25.0
@export var road_clearance_m: float = 10.0
## Parties (4 MultiMesh au plus chacune) réécrites par image au plus : part affichée, maillage,
## ombres, paramètres d'instance (étalement du coût pendant un zoom).
@export var max_part_updates: int = 96
## Semis simultanés, recalages simultanés, cellules gardées en cache.
@export var max_jobs: int = 3
@export var max_ground_jobs: int = 1
@export var max_cells: int = 96
## Instances gardées (mémoire : 64 octets par instance côté processeur et autant côté GPU) :
## au-delà, les cellules hors champ les plus anciennes sont libérées.
@export var max_stored_instances: int = 600000

static var _default: ForestDetailProfile = null


## VT3 : part [0, 1] de la couche dense à afficher à la distance du rig `distance` (avant le
## préréglage), nulle au-delà de la portée des arbres.
func fraction_at(distance: float) -> float:
	var props := MapPropScale.shared()
	var t := smoothstep(dense_full_distance, props.tree_max_distance, distance)
	return props.trees_weight(distance) * lerpf(1.0, far_share, t)


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
