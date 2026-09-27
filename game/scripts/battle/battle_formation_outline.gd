class_name BattleFormationOutline
extends Node3D

## CB-M1 : contour de formation projeté sur le relief, un `Decal` par régiment (remplace l'anneau
## jaune de sélection). États (spec CB-M, « Contour de formation ») :
## - Sélectionnée : trait plein, couleur du camp ;
## - Survolée : trait pâle ;
## - Ennemie survolée : rouge, trait pointillé ;
## - Ennemie ciblée (cible `target` d'une unité sélectionnée) : rouge pulsé ;
## - En déroute, absente (détruite, sortie du champ, en réserve) : rien.
##
## Rendu seulement : la scène fournit `get_units()`, la sélection et les régiments survolés.
## Coût par image : aucune allocation ; une décale cachée n'est pas touchée, une décale visible ne
## change de texture ou de couleur que si son état ou ses proportions changent.

enum State { NONE, SELECTED, HOVERED, ENEMY_HOVERED, ENEMY_TARGETED }

const ENEMY_RED := Color(0.86, 0.12, 0.08)
const MARGIN := 3.0  # m ajoutés au front et à la profondeur (comme l'ancien anneau)
const HEIGHT := 30.0  # m : hauteur de projection, couvre le relief sous la formation
## Textures : le petit côté fait `SHORT_PX` pixels, le grand `SHORT_PX × rapport` ; un jeu de
## rapports fixes (des deux sens) borne le nombre de textures, générées une fois à la demande.
const SHORT_PX := 96
const LINE_PX := 4
const DASH_PX := 14
const GAP_PX := 10
const RATIOS: Array[float] = [1.0, 1.5, 2.0, 3.0, 4.0, 6.0, 8.0, 10.0]
const PULSE_HZ := 1.1
const PULSE_MIN := 0.3  # opacité au creux du pulsé

## Textures partagées par toutes les scènes : Vector3i(indice du rapport, vertical, pointillé).
static var _textures: Dictionary = {}

var _side_colors: Dictionary = {}
var _player_side := "attacker"
## id -> Decal ; id -> [état, clé de texture, x, z, facing, largeur, profondeur, y].
var _decals: Dictionary = {}
var _entries: Dictionary = {}
var _targets: Array[int] = []  # cibles des unités sélectionnées (réutilisé à chaque image)
var _pulsing: Array[Decal] = []
var _pulse_time := 0.0


## État du contour d'un régiment. `unit` : dictionnaire de `get_units()` ; `selected`, `hovered`,
## `targeted` : drapeaux calculés par l'appelant ; `player_side` : camp du joueur (« ennemie » =
## tout autre camp). La sélection l'emporte sur le survol ; la cible sur le survol.
static func outline_state(unit: Dictionary, selected: bool, hovered: bool, targeted: bool, player_side: String) -> int:
	if not bool(unit.get("present", true)) or str(unit.get("state", "")) == "routing":
		return State.NONE
	if str(unit.get("side", "")) != player_side:
		if targeted:
			return State.ENEMY_TARGETED
		if hovered:
			return State.ENEMY_HOVERED
		return State.NONE
	if selected:
		return State.SELECTED
	if hovered:
		return State.HOVERED
	return State.NONE


func setup(side_colors: Dictionary, player_side: String) -> void:
	name = "FormationOutlines"
	_side_colors = side_colors
	_player_side = player_side


## Met les décales à jour à partir de `get_units()`, de la sélection et des régiments survolés.
func update(units: Array, selected: Array, hovered: Array) -> void:
	_targets.clear()
	for unit in units:
		if selected.has(int(unit["id"])):
			var target := int(unit.get("target", -1))
			if target >= 0:
				_targets.append(target)
	var pulsing_dirty := false
	for unit in units:
		var id := int(unit["id"])
		var decal: Decal = _decals.get(id)
		if decal == null:
			decal = _make_decal(id)
		var entry: Array = _entries[id]
		var state := outline_state(unit, selected.has(id), hovered.has(id), _targets.has(id), _player_side)
		var state_changed := state != int(entry[0])
		if state_changed:
			pulsing_dirty = pulsing_dirty or state == State.ENEMY_TARGETED or int(entry[0]) == State.ENEMY_TARGETED
			entry[0] = state
		if state == State.NONE:
			if decal.visible:
				decal.visible = false
			continue
		var x := float(unit["x"])
		var z := float(unit["z"])
		var y := float(unit["y"])
		var facing := float(unit["facing"])
		var width := float(unit["width"]) + MARGIN
		var depth := float(unit["depth"]) + MARGIN
		if not decal.visible or x != float(entry[2]) or z != float(entry[3]) or y != float(entry[7]):
			decal.position = Vector3(x, y, z)
			entry[2] = x
			entry[3] = z
			entry[7] = y
		if not decal.visible or facing != float(entry[4]):
			decal.rotation = Vector3(0, facing, 0)
			entry[4] = facing
		var resized := absf(width - float(entry[5])) > 0.25 or absf(depth - float(entry[6])) > 0.25
		if resized:
			decal.size = Vector3(width, HEIGHT, depth)
			entry[5] = width
			entry[6] = depth
		if state_changed or resized:
			var key := texture_key(width, depth, state == State.ENEMY_HOVERED)
			if key != entry[1]:
				entry[1] = key
				var texture := _texture(key)
				decal.texture_albedo = texture
				decal.texture_emission = texture
		if state_changed:
			decal.modulate = _color(state, str(unit["side"]))
		decal.visible = true
	if pulsing_dirty:
		_pulsing.clear()
		for id in _entries:
			if int(_entries[id][0]) == State.ENEMY_TARGETED:
				_pulsing.append(_decals[id])


func _process(delta: float) -> void:
	if _pulsing.is_empty():
		return
	_pulse_time += delta
	var wave := 0.5 + 0.5 * sin(_pulse_time * TAU * PULSE_HZ)
	var alpha := lerpf(PULSE_MIN, 1.0, wave)
	for decal in _pulsing:
		decal.modulate = Color(ENEMY_RED, alpha)


## Nombre de décales créées (tests).
func decal_count() -> int:
	return _decals.size()


## Décale du régiment `id`, ou null (tests, captures).
func decal_of(id: int) -> Decal:
	return _decals.get(id)


## État courant du contour du régiment `id` (tests).
func state_of(id: int) -> int:
	return int(_entries[id][0]) if _entries.has(id) else State.NONE


func _make_decal(id: int) -> Decal:
	var decal := Decal.new()
	decal.name = "Outline%d" % id
	decal.visible = false
	decal.size = Vector3(1, HEIGHT, 1)
	decal.albedo_mix = 1.0
	decal.emission_energy = 0.8
	decal.upper_fade = 0.05
	decal.lower_fade = 0.05
	decal.normal_fade = 0.0
	add_child(decal)
	_decals[id] = decal
	_entries[id] = [State.NONE, Vector3i(-1, -1, -1), 0.0, 0.0, 0.0, -1.0, -1.0, 0.0]
	return decal


func _color(state: int, side: String) -> Color:
	var livery: Color = _side_colors.get(side, Color(0.9, 0.8, 0.3))
	match state:
		State.SELECTED:
			return Color(livery, 1.0)
		State.HOVERED:
			return Color(livery.lerp(Color.WHITE, 0.6), 0.6)
		State.ENEMY_HOVERED:
			return Color(ENEMY_RED, 0.9)
		State.ENEMY_TARGETED:
			return Color(ENEMY_RED, 1.0)
	return Color.TRANSPARENT


## Clé de la texture pour une décale `width` × `depth` : rapport le plus proche (échelle
## logarithmique), sens (grand côté en largeur ou en profondeur), trait pointillé ou plein.
static func texture_key(width: float, depth: float, dashed: bool) -> Vector3i:
	var vertical := depth > width
	var ratio := maxf(width, depth) / maxf(minf(width, depth), 0.01)
	var best := 0
	for i in RATIOS.size():
		if absf(log(RATIOS[i]) - log(ratio)) < absf(log(RATIOS[best]) - log(ratio)):
			best = i
	return Vector3i(best, 1 if vertical else 0, 1 if dashed else 0)


static func _texture(key: Vector3i) -> ImageTexture:
	if _textures.has(key):
		return _textures[key]
	var long_px := int(round(SHORT_PX * RATIOS[key.x]))
	var size := Vector2i(SHORT_PX, long_px) if key.y == 1 else Vector2i(long_px, SHORT_PX)
	var texture := ImageTexture.create_from_image(outline_image(size, key.z == 1))
	_textures[key] = texture
	return texture


## Image du contour (blanc opaque sur fond transparent, teintée par `modulate`) : cadre de
## `LINE_PX` pixels, plein ou en tirets de `DASH_PX` séparés de `GAP_PX`.
static func outline_image(size: Vector2i, dashed: bool) -> Image:
	var image := Image.create(size.x, size.y, false, Image.FORMAT_RGBA8)
	image.fill(Color(1, 1, 1, 0))
	var white := Color(1, 1, 1, 1)
	if not dashed:
		image.fill_rect(Rect2i(0, 0, size.x, LINE_PX), white)
		image.fill_rect(Rect2i(0, size.y - LINE_PX, size.x, LINE_PX), white)
		image.fill_rect(Rect2i(0, 0, LINE_PX, size.y), white)
		image.fill_rect(Rect2i(size.x - LINE_PX, 0, LINE_PX, size.y), white)
	else:
		var period := DASH_PX + GAP_PX
		var x := 0
		while x < size.x:
			var run := mini(DASH_PX, size.x - x)
			image.fill_rect(Rect2i(x, 0, run, LINE_PX), white)
			image.fill_rect(Rect2i(x, size.y - LINE_PX, run, LINE_PX), white)
			x += period
		var z := 0
		while z < size.y:
			var run := mini(DASH_PX, size.y - z)
			image.fill_rect(Rect2i(0, z, LINE_PX, run), white)
			image.fill_rect(Rect2i(size.x - LINE_PX, z, LINE_PX, run), white)
			z += period
	image.generate_mipmaps()
	return image
