class_name BattleFormationOutline
extends Node3D

const _Access := preload("res://scripts/battle/battle_access.gd")

## Contour de formation projeté sur le relief, un `Decal` par régiment (remplace l'anneau
## jaune de sélection). États (spec CB-M, « Contour de formation ») :
## - Sélectionnée : trait plein, or pâle teinté du camp (PO4) ;
## - Survolée : trait pâle ;
## - Ennemie survolée : rouge, trait pointillé ;
## - Ennemie ciblée (cible `target` d'une unité sélectionnée) : rouge pulsé ;
## - Au corps à corps (RX batvis), sans autre état : trait fin de la livrée du camp (rouge pâle pour
##   l'ennemi), pour lire les régiments dans la mêlée ;
## - En déroute, absente (détruite, sortie du champ, en réserve) : rien.
##
## Rendu seulement : la scène fournit `get_units()`, la sélection et les régiments survolés.
## Coût par image : aucune allocation ; une décale cachée n'est pas touchée, une décale visible ne
## change de texture ou de couleur que si son état ou sa taille (arrondie) changent.
##
## Piège (corrigé) : l'émission d'une décale Godot s'ajoute sans tenir compte de l'alpha de
## l'albédo ; le fond transparent de la texture doit donc être NOIR (0, 0, 0, 0), sinon tout le
## rectangle émet la couleur de `modulate` (rectangle plein au lieu d'un cadre).

enum State { NONE, SELECTED, HOVERED, ENEMY_HOVERED, ENEMY_TARGETED, ENGAGED }

## PO4 (bible DA, ADR 0097) : liseré or pâle mince, jamais de jaune pur ; rouge garance pour
## l'ennemi. La livrée du camp ne teinte plus que légèrement l'or (`LIVERY_TINT`).
const ENEMY_RED := Color(0.8, 0.16, 0.11)
const PALE_GOLD := Color(0.93, 0.84, 0.6)
const LIVERY_TINT := 0.2
const MARGIN := 3.0  # m ajoutés au front et à la profondeur (comme l'ancien anneau)
const HEIGHT := 30.0  # m : hauteur de projection, couvre le relief sous la formation
## RX restes : au-dessus d'une eau plus profonde que `WET_DEPTH` (m), la boîte de projection se
## réduit à `WET_BOX` m autour de la surface de l'eau : seule la surface (transparente) reçoit le
## trait, pas le lit de la rivière vu à travers.
const WET_DEPTH := 0.4
const WET_BOX := 0.5
## Textures à échelle fixe : `PX_PER_M` pixels par mètre, taille de décale arrondie au pas
## `SIZE_STEP` (m) ; le trait fait donc `LINE_M` mètres quelle que soit la formation.
const PX_PER_M := 6
const SIZE_STEP := 2.0
const MAX_SIZE := 300.0  # m : borne de la texture (1800 px)
const LINE_M := 0.5
const DASH_M := 4.0
const GAP_M := 2.5
const PULSE_HZ := 1.1
const PULSE_MIN := 0.35  # opacité au creux du pulsé
const EMISSION := 0.7  # le trait reste lisible dans l'ombre et au crépuscule

## Textures partagées par toutes les scènes : Vector3i(pas en largeur, pas en profondeur,
## pointillé) -> [albédo, émission], générées une fois à la demande.
static var _textures: Dictionary = {}

var _side_colors: Dictionary = {}
## Profondeur d'eau (m) au point (x, z) ; invalide = pas de rivière (tests).
var _water_depth: Callable
var _water_surface: Callable
var _player_side := "attacker"
## id -> Decal ; id -> [état, clé de texture, x, z, facing, y].
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


## `bed_at(x, z)` : hauteur du lit ; `surface_at(x, z)` : hauteur de la surface (eau comprise).
func setup(side_colors: Dictionary, player_side: String, bed_at: Callable = Callable(), surface_at: Callable = Callable()) -> void:
	name = "FormationOutlines"
	_water_depth = bed_at
	_water_surface = surface_at
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
		if state == State.NONE and bool(unit.get("present", true)) and str(unit.get("state", "")) == "melee":
			state = State.ENGAGED
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
		if not decal.visible or x != float(entry[2]) or z != float(entry[3]) or y != float(entry[5]):
			decal.position = Vector3(x, y, z)
			entry[6] = true  # boîte à reprendre (déplacement)
			entry[2] = x
			entry[3] = z
			entry[5] = y
		if not decal.visible or facing != float(entry[4]):
			decal.rotation = Vector3(0, facing, 0)
			entry[4] = facing
		var key := texture_key(float(unit["width"]) + MARGIN, float(unit["depth"]) + MARGIN, state == State.ENEMY_HOVERED)
		var key_changed: bool = key != entry[1]
		if key_changed:
			entry[1] = key
			decal.size = Vector3(key.x * SIZE_STEP, HEIGHT, key.y * SIZE_STEP)
			var pair := _texture_pair(key)
			decal.texture_albedo = pair[0]
			decal.texture_emission = pair[1]
		if bool(entry[6]) or key_changed:
			entry[6] = false
			fit_to_water(decal, _water_depth, _water_surface)
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
		decal.modulate = Color(_Access.enemy_red(ENEMY_RED), alpha)


## Boîte de projection de `decal` (déjà posée, taille en x/z fixée) : normale (`HEIGHT`) sur terre
## ou en eau peu profonde ; réduite autour de la surface de l'eau au-dessus d'un lit profond.
static func fit_to_water(decal: Decal, bed_at: Callable, surface_at: Callable) -> void:
	var height := HEIGHT
	if bed_at.is_valid() and surface_at.is_valid():
		var x := decal.position.x
		var z := decal.position.z
		var surface := float(surface_at.call(x, z))
		if surface - float(bed_at.call(x, z)) > WET_DEPTH:
			decal.position.y = surface
			height = WET_BOX
	decal.size = Vector3(decal.size.x, height, decal.size.z)


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
	decal.emission_energy = EMISSION
	decal.upper_fade = 0.05
	decal.lower_fade = 0.05
	decal.normal_fade = 0.0
	decal.cull_mask = BattleTerrain.DECAL_LAYER  # Sol seulement, jamais les figurines
	add_child(decal)
	_decals[id] = decal
	_entries[id] = [State.NONE, Vector3i(-1, -1, -1), 0.0, 0.0, 0.0, 0.0, true]
	return decal


func _color(state: int, side: String) -> Color:
	var livery: Color = _side_colors.get(side, Color(0.9, 0.8, 0.3))
	match state:
		State.SELECTED:
			# Or pâle à peine teinté de la livrée (une livrée sombre disparaît sur l'herbe).
			return Color(PALE_GOLD.lerp(livery, LIVERY_TINT), 1.0)
		State.HOVERED:
			return Color(PALE_GOLD.lerp(Color.WHITE, 0.4), 0.6)
		State.ENEMY_HOVERED:
			return Color(_Access.enemy_red(ENEMY_RED), 0.95)
		State.ENEMY_TARGETED:
			return Color(_Access.enemy_red(ENEMY_RED), 1.0)
		State.ENGAGED:
			if side == _player_side:
				return Color(livery.lerp(Color.WHITE, 0.45), 0.7)
			return Color(_Access.enemy_red(ENEMY_RED).lerp(Color.WHITE, 0.25), 0.6)
	return Color.TRANSPARENT


## Clé de texture d'une décale `width` × `depth` (m, marge comprise) : taille arrondie au pas
## `SIZE_STEP` supérieur (bornée à `MAX_SIZE`), trait pointillé ou plein.
static func texture_key(width: float, depth: float, dashed: bool) -> Vector3i:
	var steps_x := clampi(ceili(width / SIZE_STEP), 1, int(MAX_SIZE / SIZE_STEP))
	var steps_z := clampi(ceili(depth / SIZE_STEP), 1, int(MAX_SIZE / SIZE_STEP))
	return Vector3i(steps_x, steps_z, 1 if dashed else 0)


## [albédo, émission] : même cadre ; fond transparent blanc pour l'albédo (les mipmaps ne
## foncent pas le trait au loin), noir pour l'émission (ajoutée sans l'alpha, voir en tête).
static func _texture_pair(key: Vector3i) -> Array:
	if _textures.has(key):
		return _textures[key]
	var px := int(SIZE_STEP) * PX_PER_M
	var size := Vector2i(key.x * px, key.y * px)
	var pair := [
		ImageTexture.create_from_image(outline_image(size, key.z == 1, Color(1, 1, 1, 0))),
		ImageTexture.create_from_image(outline_image(size, key.z == 1, Color(0, 0, 0, 0))),
	]
	_textures[key] = pair
	return pair


## Image du contour à `PX_PER_M` px/m : cadre blanc opaque de `LINE_M` m (teinté par
## `modulate`), plein ou en tirets, sur le fond transparent `background`.
static func outline_image(size: Vector2i, dashed: bool, background: Color = Color(0, 0, 0, 0)) -> Image:
	var image := Image.create(size.x, size.y, false, Image.FORMAT_RGBA8)
	image.fill(background)
	var white := Color(1, 1, 1, 1)
	var line := int(LINE_M * PX_PER_M)
	if not dashed:
		image.fill_rect(Rect2i(0, 0, size.x, line), white)
		image.fill_rect(Rect2i(0, size.y - line, size.x, line), white)
		image.fill_rect(Rect2i(0, 0, line, size.y), white)
		image.fill_rect(Rect2i(size.x - line, 0, line, size.y), white)
	else:
		var dash := int(DASH_M * PX_PER_M)
		var period := dash + int(GAP_M * PX_PER_M)
		var x := 0
		while x < size.x:
			var run := mini(dash, size.x - x)
			image.fill_rect(Rect2i(x, 0, run, line), white)
			image.fill_rect(Rect2i(x, size.y - line, run, line), white)
			x += period
		var z := 0
		while z < size.y:
			var run := mini(dash, size.y - z)
			image.fill_rect(Rect2i(0, z, line, run), white)
			image.fill_rect(Rect2i(size.x - line, z, line, run), white)
			z += period
	image.generate_mipmaps()
	return image
