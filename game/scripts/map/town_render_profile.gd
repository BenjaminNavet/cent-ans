class_name TownRenderProfile
extends Resource

## Lot ZG6 (ADR 0036) : réglages du rendu des villes ordinaires à l'échelle réelle
## (`res://resources/town_render.tres`). Distances en unités monde (1 unité ≈ 719 m), mesurées
## de la caméra. Les facteurs par préréglage de qualité (`RenderQuality`, lot PF1) multiplient
## portées et rayon de chargement. Purement visuel.

## Maisons du kit (détail) jusqu'à cette distance, puis blocs simples jusqu'à `block_range`.
@export var detail_range: float = 1.6
@export var block_range: float = 14.0
## Rayon de chargement autour de la caméra : clamp(`stream_factor` × distance du rig,
## `stream_min`, `stream_max`) ; déchargement au-delà de `unload_factor` × ce rayon.
@export var stream_min: float = 4.0
@export var stream_factor: float = 2.5
@export var stream_max: float = 18.0
@export var unload_factor: float = 1.3
## Actif sous ce poids du palier vallée (`ZoomTiers.valley_weight`) : au-dessus, maquettes.
@export var min_valley_weight: float = 0.5
## Plans calculés en parallèle (fils de travail) et budget de construction par image (ms).
@export var max_plan_jobs: int = 2
@export var build_budget_ms: float = 3.0
## Délai (ms) sans nouvelle page de relief avant de recalculer les hauteurs d'une ville.
@export var reground_settle_ms: int = 1200
## Plans gardés en cache après déchargement (reconstruction rapide).
@export var plan_cache: int = 24
## Facteurs par niveau de `RenderQuality` : portée du détail, des blocs, rayon de chargement,
## ombres des maisons détaillées et des blocs.
@export var quality: Dictionary = {
	"low": {"detail": 0.55, "block": 0.6, "stream": 0.6, "detail_shadows": false, "block_shadows": false},
	"medium": {"detail": 0.8, "block": 0.8, "stream": 0.8, "detail_shadows": true, "block_shadows": false},
	"high": {"detail": 1.0, "block": 1.0, "stream": 1.0, "detail_shadows": true, "block_shadows": false},
	"ultra": {"detail": 1.4, "block": 1.3, "stream": 1.2, "detail_shadows": true, "block_shadows": true},
}


static func load_default() -> TownRenderProfile:
	var path := "res://resources/town_render.tres"
	if ResourceLoader.exists(path):
		var loaded := load(path) as TownRenderProfile
		if loaded != null:
			return loaded
	return TownRenderProfile.new()


## Facteurs du niveau de qualité `level` (repli « high »).
func factors(level: String) -> Dictionary:
	return quality.get(level, quality.get("high", {}))
