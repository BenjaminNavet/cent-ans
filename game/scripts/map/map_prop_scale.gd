class_name MapPropScale
extends Resource

## Lot SZ4 (suites du zoom ZG, ADR 0036) : échelle des accessoires dessinés à l'échelle de la carte
## (arbres, moulins, hameaux, fumées ; lot SZ4b : maquettes des colonies) selon la distance du rig
## de caméra (`res://resources/map_prop_scale.tres`). Purement visuel.
##
## Historique : SZ4 / SZ4b grossissaient tous les accessoires d'une exagération commune E(d)
## (jusqu'à ×125 au-dessus de `shrink_start`, 1 sous `shrink_end`). VT / VT2 / VT3 (ADR 0138) :
## hameaux, moulins, panaches de cheminée, arbres et touffes d'herbe (`GroundClutter`) sont à
## l'échelle 1:1 à toute distance (échelle constante), coupés au-delà d'une portée de visibilité
## quand ils deviennent sous-pixel ; la forêt lointaine est portée par le terrain (canopée de
## `terrain.gdshader`). Seuls les **incendies** (événement de jeu) gardent l'exagération :
## `fire_scale(d)` = min(1, `fire_ratio` × E(d)), E = `max_exaggeration`^(1 − avancement),
## avancement en `smoothstep` sur le logarithme de la distance entre `shrink_start` et `shrink_end`.

## Incendies : distance du rig (unités monde, 1 unité ≈ 719 m) au-dessus de laquelle ils gardent
## leur taille de carte, et distance en deçà de laquelle ils sont à leur taille réelle.
@export var shrink_start: float = 28.0
@export var shrink_end: float = 8.0
## Exagération à `shrink_start` (les incendies y sont encore à leur taille de carte).
@export var max_exaggeration: float = 125.0
## Taille réelle / taille carte, par famille (1 unité ≈ 719 m). Hameaux : ~2 unités de large,
## 40-60 m en vrai. Fumées d'incendie : 2 × 9 km sur la carte, ~60 × 400 m en vrai.
@export var hamlet_ratio: float = 0.03
@export var fire_ratio: float = 0.05
## VT3 : arbres à l'échelle 1:1 à toute distance (échelle constante, `tree_scale()`). Hauteurs de
## modèle (`VegetationTileJob._make_instance`, 1 = hauteur du maillage) : chêne 1,1-1,7, hêtre
## 1,35-1,95, conifère 1,3-2,1, haie 0,3-0,46 ; × 0,018 × 719 m : chêne 14-22 m, hêtre 17-25 m,
## conifère 17-27 m, haie 4-6 m (avant VT3 : 0,035, soit 28-53 m, et grossis jusqu'à ×125).
@export var tree_ratio: float = 0.018
## Portée des arbres (distance du rig, unités) : au-delà, plus aucun arbre individuel (cartes,
## maillages, imposteurs) n'est dessiné. Arbre de 20 m en 1080p, fov 55° : ≈ 29 / D px à D unités
## de la caméra, soit 1 px vers D = 29. Fondu sur `visibility_fade` (dernier 20 %).
@export var tree_max_distance: float = 30.0
## Distance caméra → arbre (métrique du shader de feuillage : horizontale + moitié de la hauteur)
## au-delà de laquelle un arbre n'est plus dessiné (≈ 1 px), éteint par graine sur
## `tree_view_fade` × cette portée.
@export var tree_view_range: float = 30.0
@export var tree_view_fade: float = 0.35
## Distance du rig au-delà de laquelle les arbres ne portent plus d'ombre (arbre de 20 m ≈ 3 px
## au point visé à d = 10 en 1080p).
@export var tree_shadow_distance: float = 12.0
## HC1 (ADR 0161) : arbres généralisés (`map.tree_style = generalised`). Taille monde constante
## et grossie, indépendante de la distance de caméra : un arbre représente un bois. Hauteur monde
## (unités, 1 unité = 1 px carte ≈ 0,72 km) d'un feuillu adulte, c'est-à-dire d'un arbre de
## `generalised_reference_height` unités de modèle (chêne 1,1-1,7) ; maquettes GC : village 2,4,
## bourg 4,5, ville 8 unités.
@export var generalised_tree_height: float = 1.5
@export var generalised_reference_height: float = 1.4
## Variation relative de taille autour de la hauteur de l'essence (± cette part).
@export var generalised_size_variation: float = 0.25
## Lisières des massifs (uniformes `edge_height` / `edge_thin` du feuillage, 0,62 / 0,45 en 1:1) :
## hauteur relative et part des arbres retirés sur la rampe de la couverture forestière.
@export var generalised_edge_height: float = 0.85
@export var generalised_edge_thin: float = 0.1
## Élargissement des houppiers (largeur des instances après le semis) : des houppiers plus ronds
## referment la canopée sans resserrer le semis.
@export var generalised_crown_widen: float = 1.25
## Pas (px carte) du semis généralisé : la canopée se referme quand il est proche de la largeur
## d'un houppier.
@export var generalised_spacing: float = 1.3
## Portée (distance du rig) des arbres généralisés, fondu sur `generalised_fade` (part finale).
@export var generalised_max_distance: float = 900.0
@export var generalised_fade: float = 0.25
## Portée de dessin autour de la caméra (même métrique que le shader de feuillage : distance
## horizontale + moitié de la hauteur de la caméra) : `generalised_view_base` +
## `generalised_view_factor` × distance du rig. Les tuiles au-delà ne sont ni semées ni dessinées.
@export var generalised_view_base: float = 120.0
@export var generalised_view_factor: float = 1.6
## Avance du centre de dessin au-delà du point visé (part de la distance du rig) et part finale
## du rayon sur laquelle les arbres s'éclaircissent.
@export var generalised_view_lead: float = 0.5
@export var generalised_view_fade: float = 0.3
## Plafond du rayon de dessin (mémoire : tuiles semées).
@export var generalised_view_max: float = 900.0
## Éclaircissement au dézoom (coût : à la distance 700 un arbre de 0,8 unité fait ≈ 1 px) : part des
## arbres gardée = `generalised_thin_start` / distance du rig, bornée entre
## `generalised_far_density` et 1 ; le shader grossit les arbres restants de 1/√part (même
## couvert, un arbre représente un bois plus grand). Taille strictement constante en deçà de
## `generalised_thin_start` ; `generalised_far_density` = 1 : constante partout.
@export var generalised_thin_start: float = 150.0
@export var generalised_far_density: float = 0.35
## Distance caméra → partie de tuile en deçà de laquelle les arbres sont en maillage détaillé
## (sans imposteurs générés GA3) ; imposteur ou maillage bas au-delà.
@export var generalised_mesh_distance: float = 60.0
## Lot DN-FORET (ADR 0216) : zone autour du point visé où les arbres sont les maillages décimés des
## modèles générés (un MultiMesh par essence) ; imposteurs au-delà. Rayon (unités carte) =
## `generalised_model_radius_factor` × distance du rig, bornés ; pas de modèles au-delà de
## `generalised_model_max_distance` (coût : ≈ 1 200 triangles par arbre, ombres comprises).
@export var generalised_model_radius_factor: float = 0.6
@export var generalised_model_radius_min: float = 8.0
@export var generalised_model_radius_max: float = 40.0
@export var generalised_model_max_distance: float = 150.0
## Distance du rig au-delà de laquelle les arbres généralisés ne portent plus d'ombre.
@export var generalised_shadow_distance: float = 70.0
## Tuiles gardées en cache (une vue stratégique en montre plus que les 64 de VT3).
@export var generalised_max_cached_tiles: int = 96
## Rayon nominal d'un houppier / hauteur d'un feuillu adulte (clairières des lieux, lacs).
@export var generalised_crown_ratio: float = 0.6
## Part du rayon de houppier ajoutée aux exclusions (lieux, fleuves, lacs, mer, routes).
@export var generalised_crown_clearance: float = 1.0
## Demi-largeur (px carte) dégagée de part et d'autre des routes principales (0 : pas de test).
@export var generalised_road_clearance: float = 0.35
## Gains appliqués aux probabilités hors forêt de `tree_species.json` (réglées pour le pas 1:1) :
## bosquets, arbres isolés et de haie, vergers, ripisylves, garrigue.
@export var generalised_grove_gain: float = 1.0
@export var generalised_isolated_gain: float = 1.0
@export var generalised_orchard_gain: float = 1.0
@export var generalised_riparian_gain: float = 1.0
@export var generalised_scrub_gain: float = 1.0
## Poids du bocage (canal de haies du masque) sur les arbres épars (`hedge_boost` de
## `tree_species.json`, 3 en 1:1) : dans ce style, pas de haies alignées sur la trame du
## parcellaire, le bocage se lit par des arbres épars plus nombreux.
@export var generalised_hedge_boost: float = 6.0
## Seuils du bruit des bosquets (0,28 / 0,42 en 1:1) et cœur planté (`grove_core`, 0,55 en 1:1) :
## plus bas = bosquets plus nombreux et plus larges.
@export var generalised_grove_low: float = 0.28
@export var generalised_grove_high: float = 0.42
@export var generalised_grove_core: float = 0.55
## VT2 (ADR 0138, addendum) : moulins, panaches de cheminée (et figurants FK, `map_scenes.json`)
## sont à l'échelle 1:1 à toute distance, comme les villes et les hameaux : échelle constante
## `*_ratio` × leur taille de modèle, plus d'exagération. Moulin (`WINDMILL_SCALE` 4,6 × modèle) :
## faîte du corps 0,36 u de modèle → ~11 m, moyeu ~9 m, ailes ~18 m d'envergure (moulin sur pivot
## médiéval : 10-12 m, ailes 18-24 m). Panache (`CHIMNEY_SIZE` 1,3 × 4, ± 20 %) : ~9 × 28 m.
@export var windmill_ratio: float = 0.0093
@export var chimney_ratio: float = 0.0098
## Opacité des panaches de cheminée (un filet de fumée de 30 m n'est qu'un voile).
@export var chimney_real_alpha: float = 0.5
## Portées de visibilité (distance du rig, unités monde) : au-delà, moulins et panaches ne sont
## plus dessinés (moins d'un à deux pixels en 1080p, fov 55°, on ne paie pas un rendu invisible).
## Moulin ~18 m hors tout : ≈ 26 / d px ; panache ~28 m de haut : ≈ 40 / d px.
@export var windmill_max_distance: float = 30.0
@export var chimney_max_distance: float = 25.0
## Part de la portée des panaches sur laquelle leur opacité s'éteint (pas d'apparition brusque).
@export var visibility_fade: float = 0.2
## Variation relative d'échelle en deçà de laquelle les instances recalculées sur le processeur
## ne sont pas réécrites (évite une réécriture par image pendant un zoom).
@export var rewrite_step: float = 0.04

static var _default: MapPropScale = null

## HC1 : style des arbres de la carte (`map.tree_style`, `--tree-style=real|generalised`).
const TREE_STYLE_REAL := "real"
const TREE_STYLE_GENERALISED := "generalised"
static var _tree_style: String = ""


## VT3 : arbres à l'échelle 1:1 à toute distance (`tree_ratio` × leur taille de modèle).
func tree_scale() -> float:
	return clampf(tree_ratio, 1e-4, 1.0)


## VT3 : poids [0, 1] des arbres individuels à la distance du rig `distance` (1 en deçà, 0 au-delà
## de `tree_max_distance`, fondu sur `visibility_fade`).
func trees_weight(distance: float) -> float:
	return range_weight(distance, tree_max_distance)


## VT3 : vrai si des arbres individuels sont dessinés à la distance du rig `distance`.
func trees_visible(distance: float) -> bool:
	return distance < tree_max_distance


## Taille à l'écran (px) d'un objet de `height_m` mètres à `camera_distance` unités de la caméra,
## pour un écran de `screen_px` px de haut et un champ vertical `fov_deg` (réglage des portées).
static func pixels_for(height_m: float, camera_distance: float, screen_px: float = 1080.0, fov_deg: float = 55.0) -> float:
	var px_per_unit := screen_px / (2.0 * maxf(camera_distance, 1e-4) * tan(deg_to_rad(fov_deg) * 0.5))
	return height_m / 719.0 * px_per_unit



## VT2 : moulins à l'échelle 1:1 à toute distance (`windmill_ratio` × leur taille de modèle).
func windmill_scale() -> float:
	return clampf(windmill_ratio, 1e-4, 1.0)


## VT (ADR 0138) : les hameaux sont à l'échelle 1:1 à toute distance (pas d'exagération) :
## toujours `hamlet_ratio` × leur taille de modèle. `distance` est gardé pour la signature commune.
func hamlet_scale(_distance: float) -> float:
	return clampf(hamlet_ratio, 1e-4, 1.0)


## VT2 : panaches de cheminée à l'échelle 1:1 à toute distance.
func chimney_scale() -> float:
	return clampf(chimney_ratio, 1e-4, 1.0)


## Facteur d'opacité des panaches de cheminée : `chimney_real_alpha`, éteint en approchant de
## `chimney_max_distance` (0 au-delà).
func chimney_alpha(distance: float) -> float:
	return chimney_real_alpha * range_weight(distance, chimney_max_distance)


## Vrai si les moulins sont dessinés à la distance `distance`.
func windmills_visible(distance: float) -> bool:
	return distance < windmill_max_distance


## Poids de visibilité (1 en deçà, 0 au-delà de `max_distance`, fondu sur `visibility_fade`).
func range_weight(distance: float, max_distance: float) -> float:
	var fade := maxf(max_distance * visibility_fade, 1e-4)
	return 1.0 - smoothstep(max_distance - fade, max_distance, distance)


## Incendies : 1 au loin (taille de carte), `fire_ratio` (taille réelle) sous `shrink_end`.
func fire_scale(distance: float) -> float:
	var progress := 1.0
	if distance >= shrink_start:
		progress = 0.0
	elif distance > shrink_end:
		progress = smoothstep(0.0, 1.0, log(shrink_start / distance) / log(shrink_start / shrink_end))
	var exaggeration := pow(maxf(max_exaggeration, 1.0), 1.0 - progress)
	return minf(1.0, clampf(fire_ratio, 1e-4, 1.0) * exaggeration)


## Vrai si l'échelle `now` s'écarte assez de `applied` pour réécrire des instances.
func needs_rewrite(applied: float, now: float) -> bool:
	if applied <= 0.0:
		return true
	return absf(now / applied - 1.0) > rewrite_step or (now == 1.0 and applied != 1.0)


## Réglages par défaut (ressource partagée), ou une instance neuve si absente.
static func shared() -> MapPropScale:
	if _default == null:
		var path := "res://resources/map_prop_scale.tres"
		if ResourceLoader.exists(path):
			_default = load(path) as MapPropScale
		if _default == null:
			_default = MapPropScale.new()
	return _default


## HC1 (ADR 0161) : style des arbres, `generalised` (défaut des données) ou `real` (arbres 1:1 de
## VT3) ; `map.tree_style` de `data/ui/campaign_map.json`, remplacé par `--tree-style=` après `--`.
static func tree_style() -> String:
	if _tree_style == "":
		_tree_style = str(ArmyFigures.map_settings().get("tree_style", TREE_STYLE_REAL))
		_tree_style = CmdArgs.value("--tree-style", _tree_style)
		if _tree_style != TREE_STYLE_GENERALISED:
			_tree_style = TREE_STYLE_REAL
	return _tree_style


## Facteur monde / modèle des arbres généralisés (`campaign_prop_scale`).
func generalised_scale() -> float:
	return clampf(generalised_tree_height / maxf(generalised_reference_height, 1e-3), 1e-4, 4.0)


## Rayon nominal d'un houppier généralisé (unités monde).
func generalised_crown_radius() -> float:
	return generalised_tree_height * generalised_crown_ratio * generalised_crown_clearance


## Échelle des arbres de la carte selon le style (`campaign_prop_scale`).
func map_tree_scale() -> float:
	return generalised_scale() if trees_generalised() else tree_scale()


## Portée de dessin autour de la caméra à la distance du rig `distance` (style généralisé).
func generalised_view_range(distance: float) -> float:
	return minf(generalised_view_base + generalised_view_factor * distance, generalised_view_max)


## Part des arbres gardée à la distance du rig `distance` (style généralisé).
func generalised_density(distance: float) -> float:
	return clampf(generalised_thin_start / maxf(distance, 1e-3), clampf(generalised_far_density, 0.05, 1.0), 1.0)


static func trees_generalised() -> bool:
	return tree_style() == TREE_STYLE_GENERALISED


## Tests : force le style ("" : relu des données au prochain appel).
static func set_tree_style(value: String) -> void:
	_tree_style = value
