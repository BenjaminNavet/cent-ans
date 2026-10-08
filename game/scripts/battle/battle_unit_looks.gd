class_name BattleUnitLooks
extends RefCounted
## Variantes de rendu des figurines par type d'unité (lot OMR R5, `data/fx/unit_looks.json`) :
## part des soldats en livrée, étoffes des vêtements non teints, robes des chevaux. Rendu
## seulement ; la figurine elle-même reste celle du champ `figure` du type d'unité.

const FX_FILE := "fx/unit_looks.json"
const MAX_COLOURS := 4
## Robes nommées (linéaires), mêmes valeurs que `COATS` de `battle_soldier_skinned.gdshader`.
const COATS := {
	"bay": Vector3(0.16, 0.07, 0.03),
	"chestnut": Vector3(0.26, 0.10, 0.03),
	"black": Vector3(0.03, 0.025, 0.022),
	"grey": Vector3(0.34, 0.33, 0.31),
	"dun": Vector3(0.36, 0.26, 0.13),
}

static var _lookup := JsonLookup.new(FX_FILE)


## Variante déclarée pour `unit_type` ({} si aucune).
static func look(unit_type: String) -> Dictionary:
	return _lookup.section("looks").get(unit_type, {})


## Étoffes d'une variante en couleurs linéaires (au plus `MAX_COLOURS`).
static func cloth_colours(entry: Dictionary) -> PackedVector3Array:
	var out := PackedVector3Array()
	for hex in entry.get("cloth", []):
		if out.size() >= MAX_COLOURS:
			break
		var c := Color.html(str(hex)).srgb_to_linear()
		out.append(Vector3(c.r, c.g, c.b))
	return out


## Robes d'une variante en couleurs linéaires (noms inconnus ignorés).
static func coat_colours(entry: Dictionary) -> PackedVector3Array:
	var out := PackedVector3Array()
	for name in entry.get("coats", []):
		if out.size() < MAX_COLOURS and COATS.has(str(name)):
			out.append(COATS[str(name)])
	return out


## Applique la variante de `unit_type` au matériau d'un régiment (sans effet si aucune).
static func apply(mat: ShaderMaterial, unit_type: String) -> void:
	var entry := look(unit_type)
	if entry.is_empty():
		return
	if entry.has("livery_share"):
		mat.set_shader_parameter("livery_share", float(entry["livery_share"]))
	var cloth := cloth_colours(entry)
	if not cloth.is_empty():
		mat.set_shader_parameter("plain_colors", _padded(cloth))
		mat.set_shader_parameter("plain_count", cloth.size())
	var coats := coat_colours(entry)
	if not coats.is_empty():
		mat.set_shader_parameter("coat_colors", _padded(coats))
		mat.set_shader_parameter("coat_count", coats.size())


static func _padded(colours: PackedVector3Array) -> PackedVector3Array:
	var out := colours.duplicate()
	while out.size() < MAX_COLOURS:
		out.append(colours[0])
	return out
