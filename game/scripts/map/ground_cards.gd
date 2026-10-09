class_name GroundCards
extends RefCounted

## TX T3 (ADR 0241) : cartes de plantes du sol de la carte de campagne, cinq par biome (bruyère et
## fougère aigle en Atlantique nord, halfa et tamaris au désert, stipe en steppe, ciste et lentisque
## au maquis…), détourées en RGBA 1024 dans un tableau de couches
## (`game/assets/textures/vegetation/tx_ground_cards_array.webp`, manifeste
## `data/art/tx_ground_cards_pack.json`, catalogue `data/art/textures/vegetation_cards.yaml`).
## `GroundClutter` pose ces cartes à la place de la touffe d'herbe unique. Un biome sans carte
## (classes 8-14 sans entrée) se replie sur son `parent` (`BiomeParents`). Sans paquet ou avec
## `--legacy-textures` : `ready()` est faux et le rendu reste celui d'avant. Purement visuel.

const PACK_FILE := "art/tx_ground_cards_pack.json"
const MAX_LAYERS := 96  # taille des tableaux du shader

static var _loaded := false
static var _pack: Dictionary = {}
static var _by_biome: Array = []  # biome (0..14) → PackedInt32Array de couches
static var _size_m := PackedFloat32Array()  # couche → taille réelle (m)
static var _albedo: Texture = null


static func clear_cache() -> void:
	_loaded = false
	_pack = {}
	_by_biome = []
	_size_m = PackedFloat32Array()
	_albedo = null


static func _ensure() -> void:
	if _loaded:
		return
	_loaded = true
	if not TextureQuality.use_tx() or not DataFile.exists(PACK_FILE):
		return
	var parsed: Variant = DataFile.read_json(PACK_FILE)
	if not parsed is Dictionary:
		return
	_pack = parsed as Dictionary
	_albedo = load(TextureQuality.texture_path(str(_pack.get("albedo", "")))) as Texture
	if _albedo == null:
		_pack = {}
		return
	_by_biome.clear()
	for i in BiomeParents.COUNT:
		_by_biome.append(PackedInt32Array())
	_size_m.resize(MAX_LAYERS)
	_size_m.fill(0.5)
	for layer_v in _pack.get("layers", []):
		var layer := layer_v as Dictionary
		if str(layer.get("role", "")) != "ground_card" or int(layer["layer"]) >= MAX_LAYERS:
			continue
		_size_m[int(layer["layer"])] = float(layer["tile_m"])
		for biome_v in layer.get("biomes", []):
			var biome := int(biome_v)
			if biome >= 0 and biome < BiomeParents.COUNT:
				(_by_biome[biome] as PackedInt32Array).append(int(layer["layer"]))


## Cartes disponibles (paquet chargé).
static func ready() -> bool:
	_ensure()
	return _albedo != null and not _pack.is_empty()


## Couches du tableau pour `biome`, avec repli sur les parents ; vide sans carte.
static func layers_for_biome(biome: int) -> PackedInt32Array:
	_ensure()
	if _by_biome.is_empty():
		return PackedInt32Array()
	var resolved := BiomeParents.resolve(biome, func(b: int) -> bool: return not (_by_biome[b] as PackedInt32Array).is_empty())
	return _by_biome[resolved]


## Taille réelle (m) de la carte de la couche `layer` (plus grande dimension de la plante).
static func size_m(layer: int) -> float:
	_ensure()
	return _size_m[clampi(layer, 0, MAX_LAYERS - 1)] if not _size_m.is_empty() else 0.5


static func albedo() -> Texture:
	_ensure()
	return _albedo


## Couche tirée de façon déterministe par `key` (par exemple la position) pour `biome`, -1 sans carte.
static func pick(biome: int, key: int) -> int:
	var layers := layers_for_biome(biome)
	if layers.is_empty():
		return -1
	return layers[posmod(key, layers.size())]
