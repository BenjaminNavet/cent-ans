class_name CloseCameraProfile
extends Resource

## Lot ZG4 (ADR 0036) : réglages de la caméra rapprochée de la carte de campagne et de
## l'exagération verticale dynamique (`res://resources/close_camera.tres`). Purement visuel.
##
## Distances en unités monde (1 unité = 1 pixel de carte 4096 = 719 m), mesurées du point visé à
## la caméra (distance du rig, `CampaignCamera.distance`).
##
## - Distance minimale par étage de pyramide (`level_min_distance[k]`, E0 → E7) : la caméra peut
##   descendre d'autant plus bas que le relief chargeable sous elle est fin. Le champ est adouci
##   dans l'espace (`min_distance_slope` : chaque unité qui sépare le point visé d'une zone plus
##   fine ajoute cette quantité à la distance minimale de la zone) pour ne jamais « pousser »
##   brutalement la caméra au bord d'une zone de détail.
## - Exagération verticale : `exaggeration_far` (×4,31 = `MapData.HEIGHT_SCALE` × 719 m) au-delà de
##   `exaggeration_far_distance`, `exaggeration_near` en deçà de `exaggeration_near_distance`,
##   interpolation lisse (smoothstep) en logarithme de la distance entre les deux. L'échelle est
##   quantifiée par paliers géométriques de `exaggeration_step` (≈ 4 %) avec hystérésis, pour que
##   les calques qui posent des objets au sol ne se recalent qu'à chaque palier.
## - Tangage : inchangé au-dessus de `pitch_reference_distance` (courbe historique du lot C6,
##   30° → 70°) ; en deçà, de plus en plus rasant jusqu'à `pitch_closest_deg` à
##   `pitch_closest_distance` (vue « à hauteur de colline »).
## - Plans de coupe : `near` ∝ distance (bornée), `far` ∝ distance + marge (horizon visible en vue
##   rasante, profondeur utile au loin).

## Distance minimale par étage E0 … E7 (E0 = tuiles 360 m, repli mer / hors pyramide).
@export var level_min_distance: PackedFloat32Array = PackedFloat32Array([22.0, 9.0, 5.0, 2.6, 1.5, 0.8, 0.5, 0.3])
## Croissance de la distance minimale par unité d'éloignement d'une zone plus fine.
@export var min_distance_slope: float = 0.45

@export var exaggeration_far: float = 4.31
@export var exaggeration_near: float = 1.5
@export var exaggeration_far_distance: float = 45.0
@export var exaggeration_near_distance: float = 0.45
## Pas géométrique de quantification de l'échelle verticale (0,04 = paliers de 4 %).
@export var exaggeration_step: float = 0.04
## Hystérésis, en fraction de palier (0,15 : il faut dépasser le milieu de 15 % du pas).
@export var exaggeration_hysteresis: float = 0.15

@export var pitch_reference_distance: float = 22.0
@export var pitch_closest_deg: float = 11.0
@export var pitch_closest_distance: float = 0.3
## Visée relevée au-dessus du point visé, en fraction de la distance, au plus près (0 à la distance de
## référence) : la vue rasante montre crêtes et horizon au lieu du sol sous la caméra.
@export var look_up_factor: float = 0.3

@export var near_factor: float = 0.02
@export var near_min: float = 0.002
@export var near_max: float = 1.0
@export var far_factor: float = 4.0
@export var far_margin: float = 700.0
@export var far_max: float = 6000.0

## Garde au sol de la caméra : fraction de la distance, au moins `clearance_min` unités.
@export var clearance_factor: float = 0.06
@export var clearance_min: float = 0.004
## Crêtes entre la caméra et le point visé : échantillons le long de la visée (au-delà de la
## fraction `occlusion_min_t` de la distance, en partant du point visé).
@export var occlusion_samples: int = 6
@export var occlusion_min_t: float = 0.25


## Réglages par défaut (`res://resources/close_camera.tres`), ou une instance neuve si absente.
static func load_default() -> CloseCameraProfile:
	var path := "res://resources/close_camera.tres"
	if ResourceLoader.exists(path):
		var loaded := load(path) as CloseCameraProfile
		if loaded != null:
			return loaded
	return CloseCameraProfile.new()


## Distance minimale pour un étage de pyramide (-1 ou hors table : E0).
func min_distance_for_level(level: int) -> float:
	if level_min_distance.is_empty():
		return 22.0
	return level_min_distance[clampi(level, 0, level_min_distance.size() - 1)]


## Distance minimale adoucie au point carte `p` : min sur les étages k de
## `level_min_distance[k] + min_distance_slope × (distance de p à la tuile d'étage k la plus
## proche)` (tuiles de la pyramide, rectangles exacts : champ continu, lipschitzien de pente
## `min_distance_slope`). `relief` : `ReliefPyramid` ou tout objet qui expose `max_level`,
## `finest_level_at(x, y)` et `has_tile(level, col, row)`.
func soft_min_distance(p: Vector2, relief: Object) -> float:
	var best := min_distance_for_level(int(relief.call("finest_level_at", p.x, p.y)))
	var slope := maxf(min_distance_slope, 1e-3)
	var top := mini(int(relief.get("max_level")), level_min_distance.size() - 1)
	for level in range(top, 0, -1):
		var floor_k := min_distance_for_level(level)
		if floor_k >= best:
			continue
		var radius := (best - floor_k) / slope
		var t := ReliefPyramid.tile_units(level)
		var lo := ReliefPyramid.tile_at(level, p.x - radius, p.y - radius)
		var hi := ReliefPyramid.tile_at(level, p.x + radius, p.y + radius)
		# Garde-fou : au plus 24 × 24 tuiles par étage (rayons de ~10 unités sur E7).
		if (hi.x - lo.x) > 24 or (hi.y - lo.y) > 24:
			var c := ReliefPyramid.tile_at(level, p.x, p.y)
			lo = c - Vector2i(12, 12)
			hi = c + Vector2i(12, 12)
		for row in range(lo.y, hi.y + 1):
			for col in range(lo.x, hi.x + 1):
				if not bool(relief.call("has_tile", level, col, row)):
					continue
				var origin := ReliefPyramid.tile_origin(level, col, row)
				var nearest := Vector2(clampf(p.x, origin.x, origin.x + t), clampf(p.y, origin.y, origin.y + t))
				best = minf(best, floor_k + slope * nearest.distance_to(p))
	return best


## Exagération verticale continue (×) pour une distance caméra.
func exaggeration_at(distance: float) -> float:
	var lo := log(maxf(exaggeration_near_distance, 1e-4))
	var hi := log(maxf(exaggeration_far_distance, exaggeration_near_distance * 1.01))
	var t := smoothstep(lo, hi, log(maxf(distance, 1e-4)))
	# Interpolation géométrique : paliers de quantification réguliers sur toute la plage.
	return exaggeration_near * pow(exaggeration_far / exaggeration_near, t)


## Échelle verticale continue (unités monde par mètre) : `MapData.HEIGHT_SCALE` en vue
## stratégique, ramenée proportionnellement à l'exagération.
func vertical_scale_at(distance: float) -> float:
	return MapData.HEIGHT_SCALE * exaggeration_at(distance) / exaggeration_far


## Échelle quantifiée (paliers géométriques de `exaggeration_step` sous `HEIGHT_SCALE`) ;
## `current` (échelle affichée) donne l'hystérésis : on ne quitte son palier qu'au-delà du milieu
## + `exaggeration_hysteresis` pas.
func quantized_scale(distance: float, current: float = -1.0) -> float:
	var step := log(1.0 + maxf(exaggeration_step, 1e-3))
	var x := log(vertical_scale_at(distance) / MapData.HEIGHT_SCALE) / step
	var index := roundf(x)
	if current > 0.0:
		var current_index := roundf(log(current / MapData.HEIGHT_SCALE) / step)
		if absf(x - current_index) < 0.5 + exaggeration_hysteresis:
			index = current_index
	var lowest := ceilf(log(exaggeration_near / exaggeration_far) / step - 0.5)
	index = clampf(index, lowest, 0.0)
	return MapData.HEIGHT_SCALE * exp(index * step)


## Tangage (degrés) sous `pitch_reference_distance` : de `reference_deg` (valeur de la courbe
## historique à la distance de référence) à `pitch_closest_deg`, en logarithme de la distance.
func close_pitch_deg(distance: float, reference_deg: float) -> float:
	var lo := log(maxf(pitch_closest_distance, 1e-4))
	var hi := log(maxf(pitch_reference_distance, pitch_closest_distance * 1.01))
	var t := clampf((log(maxf(distance, 1e-4)) - lo) / (hi - lo), 0.0, 1.0)
	# Linéaire en logarithme de la distance : 23,5° à 5 unités (vallée), 18° à 1,5, 11° à 0,3.
	return lerpf(pitch_closest_deg, reference_deg, t)


## Hauteur (unités) dont la visée est relevée au-dessus du point visé.
func look_up(distance: float) -> float:
	var lo := log(maxf(pitch_closest_distance, 1e-4))
	var hi := log(maxf(pitch_reference_distance, pitch_closest_distance * 1.01))
	var t := clampf((log(maxf(distance, 1e-4)) - lo) / (hi - lo), 0.0, 1.0)
	return look_up_factor * (1.0 - t) * distance


func near_plane(distance: float) -> float:
	return clampf(distance * near_factor, near_min, near_max)


func far_plane(distance: float) -> float:
	return minf(distance * far_factor + far_margin, far_max)


func clearance(distance: float) -> float:
	return maxf(distance * clearance_factor, clearance_min)
