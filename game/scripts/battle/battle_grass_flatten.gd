class_name BattleGrassFlatten
extends RefCounted

## Herbe couchée et herbe tachée de sang (lot BV3, rendu seulement).
## Carte RG8 à 1 m le texel sur le champ de bataille : R = herbe couchée (0-1), G = sang sur
## l'herbe (0-1). Lue par `battle_grass.gdshader` (`flatten_map`) : les touffes couchées se
## plient au ras du sol, s'éclaircissent et jaunissent, et le sang les teinte ; les flaques de
## `BattleBlood` (décalques au ras du sol) redeviennent visibles en prairie.
## Sources : empreinte des régiments en marche (plafonnée : herbe foulée, pas rasée), front des
## mêlées (couchée), corps tombés (couchée net sous le corps, sang autour).
## Complète la carte de neige / boue piétinée de B7/B8 (`BattleTerrain.update_trample`), qui ne
## vit que sur sol enneigé ou détrempé et reste propriété du terrain.

## EP1 : 1 m sur le champ standard, 2 m sur les grands champs (mémoire, envoi GPU).
var TEXEL := 1.0
## Rectangle couvert (le champ de bataille et `MARGIN` m d'abords de chaque côté, en mètres).
var RECT := Rect2(0.0, -100.0, 1200.0, 1000.0)
## Envoi de la texture au plus toutes les `STEP` secondes de bataille.
const STEP := 0.5
## Plafonds (0-255) : une troupe qui marche foule l'herbe sans la raser.
const MARCH_CAP := 150
const IDLE_CAP := 70

var texture: ImageTexture
var enabled: bool = false
## Dernier corps marqué et nombre de corps (captures, diagnostics).
var last_corpse: Vector3 = Vector3.ZERO
var corpse_marks: int = 0

var _image: Image
var _bytes := PackedByteArray()
var _w: int = 0
var _h: int = 0
var _timer: float = 0.0
var _last: Dictionary = {}  # id -> position (x, z) au dernier passage
## PB3e : empreintes tamponnées en Rust (`StampMap`).
var _map: RefCounted = null


## NT10 : marge (m) couverte de chaque côté du champ ; les régiments qui sortent du champ par un
## flanc (déroute, retraite) couchent encore l'herbe. Au-delà, le shader ne lit plus rien.
const MARGIN := 100.0


## `field` : largeur et profondeur du champ (EP1), 1200 × 800 par défaut.
func setup(field: Vector2 = Vector2(1200.0, 800.0)) -> void:
	RECT = Rect2(-MARGIN, -MARGIN, field.x + 2.0 * MARGIN, field.y + 2.0 * MARGIN)
	TEXEL = 1.0 if field.x * field.y <= 1200.0 * 800.0 * 1.5 else 2.0
	_w = int(RECT.size.x / TEXEL)
	_h = int(RECT.size.y / TEXEL)
	_bytes.resize(_w * _h * 2)
	_map = ClassDB.instantiate(&"StampMap")
	_map.call("setup", _w, _h, 2, RECT.position, TEXEL)
	_image = Image.create_from_data(_w, _h, false, Image.FORMAT_RG8, _bytes)
	texture = ImageTexture.create_from_image(_image)
	enabled = true


## Paramètres du shader d'herbe : (x, z, largeur, profondeur) du rectangle couvert.
func rect_vec() -> Vector4:
	return Vector4(RECT.position.x, RECT.position.y, RECT.size.x, RECT.size.y)


## Fait avancer le piétinement des régiments (`dt` = temps de bataille écoulé, 0 en pause) et
## envoie la carte au GPU si elle a changé.
func update(units: Array, dt: float) -> void:
	if not enabled or dt <= 0.0:
		return
	_timer += dt
	if _timer < STEP:
		return
	var step := _timer
	_timer = 0.0
	for unit in units:
		if not bool(unit.get("present", false)):
			continue
		var id := int(unit["id"])
		var pos := Vector2(float(unit["x"]), float(unit["z"]))
		var speed := pos.distance_to(_last.get(id, pos)) / step
		_last[id] = pos
		var state := str(unit.get("state", ""))
		var facing := float(unit.get("facing", 0.0))
		var half := Vector2(float(unit.get("width", 10.0)), float(unit.get("depth", 4.0))) * 0.5
		var fwd := Vector2(sin(facing), cos(facing))
		if state == "melee":
			# Front de la mêlée : une bande de part et d'autre du premier rang, couchée vite.
			var front := pos + fwd * half.y
			_stamp_box(front, facing, Vector2(half.x + 1.5, 3.0), 40, 0, 255)
			_stamp_box(pos, facing, half, 8, 0, MARCH_CAP)
		elif speed > 0.4:
			_stamp_box(pos, facing, half + Vector2(0.5, 0.5), int(clampf(speed * 4.0, 4.0, 30.0)), 0, MARCH_CAP)
		else:
			_stamp_box(pos, facing, half, 2, 0, IDLE_CAP)
	flush()


## Un corps tombe en `pos` : herbe couchée sous lui, sang autour (`blood` 0-1).
func on_corpse(pos: Vector3, kind: String, blood: float) -> void:
	if not enabled:
		return
	last_corpse = pos
	corpse_marks += 1
	var radius := 1.6 if kind == "cavalry" else 1.0
	_stamp_disc(Vector2(pos.x, pos.z), radius, 255, 0)
	if blood > 0.0:
		_stamp_disc(Vector2(pos.x, pos.z), radius * 0.9, 0, int(200.0 * blood))


## Envoie la carte au GPU si elle a changé.
func flush() -> void:
	_map.call("upload", _image, texture)


## Herbe couchée (0-1) en un point (tests, captures).
func flatten_at(x: float, z: float) -> float:
	return float(_map.call("sample", x, z, 0))


## Sang sur l'herbe (0-1) en un point.
func blood_at(x: float, z: float) -> float:
	return float(_map.call("sample", x, z, 1))



## Rectangle orienté (`half` demi-côtés, x le long du front) : ajoute `add_r` / `add_g`,
## R plafonné à `cap_r`.
func _stamp_box(center: Vector2, facing: float, half: Vector2, add_r: int, add_g: int, cap_r: int) -> void:
	_map.call("stamp_box", center, facing, half, add_r, add_g, cap_r)


## Disque à bord adouci : R et G montent vers `r_value` / `g_value` (jamais ne baissent).
func _stamp_disc(center: Vector2, radius: float, r_value: int, g_value: int) -> void:
	_map.call("stamp_disc", center, radius, r_value, g_value)
