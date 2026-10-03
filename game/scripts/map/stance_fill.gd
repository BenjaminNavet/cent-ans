class_name StanceFill
extends Node

## Lot RJ-d (ADR 0175) : lavis translucide de chaque province selon la position diplomatique de
## son CONTRÔLEUR envers le joueur — or : nous, vert : alliés et vassaux, rouge : ennemis en
## guerre, gris très léger : les autres. Le lavis suit le contrôle (une cité prise passe à l'or :
## la progression se voit) ; la possession de droit reste dite par le trait de frontière et les
## hachures de l'occupant (FR1), peintes par-dessus le lavis.
##
## Rendu dans le fragment du terrain (`stance_fill.gdshaderinc`, crochet `sf_fill` de
## terrain.gdshader et terrain_parchment.gdshader) : une petite texture par province (index
## raster), refaite seulement quand contrôleurs ou positions changent ; aucune géométrie.
## Classification des positions : `StanceCues` (ADR 0155), non dupliquée ici.
## Réglages : `data/map/stance_fill.json` (schéma `stance_fill.schema.json`).
## Option du joueur : `map/stance_fill` (Réglages › Carte). Option (après `--`) :
## `--no-stance-fill` (A/B de perf). Purement visuel.

const MAP_PATHS_SCRIPT := preload("res://scripts/map/map_paths.gd")
const TUNING_PATH := "map/stance_fill.json"
const SETTING_KEY := "map/stance_fill"

var map: Node = null  # CampaignMap (non typé : tests et maquettes)
var terrain: TerrainBuilder = null
var tuning: Dictionary = {}
## Interrupteur du joueur (Réglages) ; faux : `sf_alpha` = 0, le crochet sort au premier test.
var enabled: bool = true
## Mode de carte courant et opacité globale effective (mode × zoom × option), pour les tests.
var mode: String = "political"
var effective_alpha: float = 0.0

var _cli_disabled: bool = false
var _texture: ImageTexture = null
var _image: Image = null
var _colors := PackedColorArray()
var _last_distance: float = -1.0


func setup(_campaign_map: Node, _terrain_builder: TerrainBuilder = null) -> void:
	pass


## Réglages de `data/map/stance_fill.json` (dictionnaire vide si absent ou illisible).
static func load_tuning() -> Dictionary:
	return {}


## Couleur de lavis d'une catégorie de position (`StanceCues` : self, enemy, friend, other) :
## rgb sRGB, a = opacité de la catégorie. Fonction pure (tests).
static func fill_color(_cue: String, _fill_tuning: Dictionary, _cue_tuning: Dictionary) -> Color:
	return Color(0, 0, 0, 0)


## Couleurs par province (index raster - 1) d'après les contrôleurs. Fonction pure (tests).
static func colors_for(_controllers: PackedStringArray, _player: String, _stances: Dictionary, _fill_tuning: Dictionary, _cue_tuning: Dictionary) -> PackedColorArray:
	return PackedColorArray()


## Contrôleurs et positions depuis la simulation ; texture refaite seulement si changée.
func refresh() -> bool:
	return false


## Chaque image : distance caméra (unités carte) ; atténuation de près et mode de carte.
func update_view(_distance: float) -> void:
	pass


func set_enabled(value: bool) -> void:
	enabled = value
