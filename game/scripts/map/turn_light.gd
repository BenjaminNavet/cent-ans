class_name TurnLight
extends Node

## Lot CM2 : cycle jour et nuit léger en fin de tour. Pendant le tour des autres factions
## (bandeau « Tour des autres factions » du lot U5 affiché), le soleil descend vers l'ouest et
## dore la carte (soir) ; au retour du joueur, une aube brève ramène la lumière du jour. Jamais
## de nuit noire : l'énergie baisse d'au plus `dusk_energy_drop`, la carte reste lisible.
## Purement visuel. `--dusk` (après `--`) fige la lumière du soir (captures).
##
## Chantier PO5 (ADR 0097) : teinte lissée (plus de saut de couleur quand le tour revient au
## joueur au milieu du crépuscule : la teinte glisse du soir vers l'aube) et cartouche de la
## nouvelle saison (`SeasonBanner`, « Printemps 1338 », 1,2 s) quand la date de la barre
## supérieure change, une fois le bandeau « Tour des autres factions » retiré.

## Soleil du soir : hauteur, azimut (d'où vient la lumière), couleur, baisse d'énergie.
@export var dusk_elevation_deg: float = 14.0
@export var dusk_azimuth_deg: float = 255.0
@export var dusk_color: Color = Color(1.0, 0.72, 0.42)
@export var dusk_energy_drop: float = 0.18
## Teinte de l'aube (retour du joueur), plus rosée.
@export var dawn_color: Color = Color(1.0, 0.84, 0.74)
@export var fade_in_s: float = 0.7
@export var fade_out_s: float = 1.6
## PO5 : taux (1/s) de glissement de la teinte chaude entre soir et aube.
@export var tint_rate: float = 3.0

## 0 = jour, 1 = soir doré.
var dusk: float = 0.0

var _sun: DirectionalLight3D
var _banner: Control
var _base_basis: Basis
var _base_color: Color
var _base_energy: float = 1.0
var _active: bool = false
var _returning: bool = false
var _forced: bool = false
## PO5 : teinte chaude courante (lissée), date suivie et cartouche de saison.
var _warm: Color = Color(1.0, 0.72, 0.42)
var _date_label: Label
var _season_text: String = ""
var season_banner: SeasonBanner


func setup(map: Node) -> void:
	_sun = map.get_node_or_null("Sun") as DirectionalLight3D
	var ui: Node = map.get("ui")
	if ui != null:
		_banner = ui.get("turn_banner") as Control
		_date_label = ui.get("date_label") as Label
		if ui is Node:
			season_banner = SeasonBanner.new()
			(ui as Node).add_child(season_banner)
	_season_text = season_of_label()
	_forced = OS.get_cmdline_user_args().has("--dusk")
	_warm = dusk_color


## Saison et année affichées par la barre supérieure (« Printemps 1338 »), "" sans barre.
func season_of_label() -> String:
	if _date_label == null or not is_instance_valid(_date_label):
		return ""
	return _date_label.text.get_slice(" — ", 0).strip_edges()


func _process(delta: float) -> void:
	_watch_season()
	if _sun == null:
		return
	var banner_up := _forced or (_banner != null and _banner.visible)
	if banner_up and not _active:
		# Début du tour des autres : mémorise la lumière du jour (saison, météo comprises).
		_active = true
		_returning = false
		if dusk <= 0.0:
			_base_basis = _sun.basis
			_base_color = _sun.light_color
			_base_energy = _sun.light_energy
			_warm = dusk_color
	elif not banner_up and _active:
		_active = false
		_returning = true
	if not _active and not _returning:
		return
	var target := 1.0 if _active else 0.0
	var speed := 1.0 / maxf(fade_in_s if _active else fade_out_s, 0.01)
	dusk = move_toward(dusk, target, speed * delta) if not _forced else 1.0
	_warm = _warm.lerp(dusk_color if _active else dawn_color, 1.0 - exp(-tint_rate * delta))
	_apply()
	if _returning and dusk <= 0.0:
		_returning = false
		_sun.basis = _base_basis
		_sun.light_color = _base_color
		_sun.light_energy = _base_energy


func _apply() -> void:
	var t := smoothstep(0.0, 1.0, dusk)
	var elevation := deg_to_rad(dusk_elevation_deg)
	var azimuth := deg_to_rad(dusk_azimuth_deg)
	var toward_sun := Vector3(sin(azimuth) * cos(elevation), sin(elevation), -cos(azimuth) * cos(elevation))
	var dusk_basis := Basis.looking_at(-toward_sun.normalized(), Vector3.UP)
	_sun.basis = _base_basis.slerp(dusk_basis, t)
	# Au retour, la lumière passe par l'aube rosée avant le jour (teinte lissée, PO5).
	_sun.light_color = _base_color.lerp(_warm, t)
	_sun.light_energy = _base_energy * (1.0 - dusk_energy_drop * t)


## PO5 : annonce la nouvelle saison quand la date change, une fois le bandeau des autres
## factions retiré (le cartouche arrive avec l'aube).
func _watch_season() -> void:
	var text := season_of_label()
	if text == "" or text == _season_text:
		return
	if _banner != null and _banner.visible and not _forced:
		return
	_season_text = text
	if season_banner != null:
		season_banner.announce(text)
