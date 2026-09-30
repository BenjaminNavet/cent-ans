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
## ADR 0138 : les villes 1:1 (ZG6, VH) sont actives sous cette distance du rig (unités) ; au-delà,
## le lointain est rendu par `TownFarLayer`. Hystérésis de `rig_hysteresis` (part) à la sortie.
@export var max_rig_distance: float = 16.0
@export var rig_hysteresis: float = 0.1
## Lot SZ4 : vu de loin, le sol bâti prend la teinte moyenne des toits du kit (imposteur des
## maisons devenues sous-pixel ; sans lui, une ville vue à 4 km n'était qu'un disque de terre
## battue). Fondu selon la distance caméra → sol (unités) de `roofscape_near` à `roofscape_far` ;
## `roofscape_strength` : part maximale ; `roofscape_cell_m` : taille d'un « toit » (variation de
## couche et de teinte) ; `roofscape_gain` : luminosité des toits moyens.
@export var roofscape_near: float = 1.2
@export var roofscape_far: float = 3.5
@export var roofscape_strength: float = 0.85
@export var roofscape_cell_m: float = 9.0
@export var roofscape_gain: float = 1.0
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
