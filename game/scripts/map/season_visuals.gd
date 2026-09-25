class_name SeasonVisuals
extends RefCounted

## Lot CV1 : saison visible sur la carte de campagne (rendu seulement). La saison vient du pont
## (`CampaignSim.get_date_label`, « Printemps 1337 ») ; elle est publiée dans le paramètre de
## shader global `campaign_season` (poids printemps, été, automne, hiver, somme 1), lu par
## `terrain.gdshader` (via `campaign_life.gdshaderinc`), `foliage.gdshader` et la surcouche de
## neige des maquettes. D'un tour à l'autre, les poids glissent en `transition_seconds`.

const SEASONS: Array[String] = ["spring", "summer", "autumn", "winter"]
const GLOBAL_PARAM := &"campaign_season"

var transition_seconds: float = 2.5
## Poids affichés et visés (x printemps, y été, z automne, w hiver).
var weights: Vector4 = Vector4(1, 0, 0, 0)
var target: Vector4 = Vector4(1, 0, 0, 0)
var season: String = "spring"
var _from: Vector4 = Vector4(1, 0, 0, 0)
var _t: float = 1.0


static func weights_of(season_name: String) -> Vector4:
	var index := SEASONS.find(season_name)
	var result := Vector4.ZERO
	result[maxi(index, 0)] = 1.0
	return result


## Saison visée ; `instant` saute la transition (chargement, captures).
func set_season(season_name: String, instant: bool = false) -> void:
	if not SEASONS.has(season_name):
		return
	var wanted := weights_of(season_name)
	season = season_name
	if instant:
		weights = wanted
		target = wanted
		_t = 1.0
		_publish()
		return
	if wanted == target:
		return
	_from = weights
	target = wanted
	_t = 0.0


## Saison lue dans le libellé de date du pont ("" si inconnue).
static func season_from_label(date_label: String) -> String:
	return RichTooltip.season_of(date_label)


func update(delta: float) -> void:
	if _t >= 1.0:
		return
	_t = minf(_t + delta / maxf(transition_seconds, 0.01), 1.0)
	var s := _t * _t * (3.0 - 2.0 * _t)
	weights = _from.lerp(target, s)
	_publish()


## Neige visible (0-1) : hiver plein, un peu en fin d'automne / début de printemps.
func snow_amount() -> float:
	return clampf(weights.w + 0.2 * weights.z + 0.25 * weights.x, 0.0, 1.0)


func _publish() -> void:
	RenderingServer.global_shader_parameter_set(GLOBAL_PARAM, weights)
