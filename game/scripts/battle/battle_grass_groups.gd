class_name BattleGrassGroups
extends RefCounted

## TX T3 (ADR 0241) : herbe de bataille par groupe de biomes. Cinq groupes (vert : biomes 1, 2, 10-13 ;
## sec : 3, 7, 14 ; steppique : 4, 8 ; alpin : 6 ; arctique : 5, 9) de trois cartes détourées
## (`grass_blades_<groupe>`, `grass_tufts_<groupe>`, `grass_clump_<groupe>`, catalogue
## `data/art/textures/vegetation_cards.yaml`, paquet `data/art/tx_battle_grass_pack.json`).
## `BattleVegetation` prend les cartes du groupe du biome de la bataille à la place des cartes
## uniques d'avant. Atlas FA7 (herbe de vrais brins, réglée à l'œil) : gardé tel quel pour le groupe
## vert ; hors vert, ses cases d'herbe haute et folle sont remplacées par les cartes du groupe
## (`fa_atlas`), les cases de chaume et de blé restent. Sans paquet ou `--legacy-textures` : `cards`
## renvoie {} et l'herbe reste celle d'avant. Purement visuel.

const PACK_FILE := "art/tx_battle_grass_pack.json"
const PROVINCES_FILE := "fx/battle_province_biomes.json"
const BIOME_FLAG := "--battle-biome"
const DEFAULT_BIOME := 2
## Cases de l'atlas FA7 remplacées hors du groupe vert (variantes d'herbe haute, à épis, folle),
## carte du groupe qui les remplit (rôle). Les cases de chaume (`short_a`/`short_b`) et de blé
## (`wheat_*`) sont conservées.
const FA_CELLS := {
	"tall_a": "grass_blades",
	"tall_b": "grass_clump",
	"seed_a": "grass_tufts",
	"seed_b": "grass_blades",
	"wild_a": "grass_tufts",
	"wild_b": "grass_clump",
	"short_c": "grass_tufts",
	"tall_c": "grass_blades",
}

static var _pack: Dictionary = {}
static var _loaded := false
static var _fa_cache: Dictionary = {}  # groupe → {"texture", "luma"}


static func clear_cache() -> void:
	_pack = {}
	_loaded = false
	_fa_cache = {}


## Biome (1..14) de la bataille : `--battle-biome N` (captures), sinon celui de la province
## (`fx/battle_province_biomes.json`), sinon le biome par défaut.
static func biome_for(province_id: String) -> int:
	var forced := CmdArgs.value(BIOME_FLAG, "")
	if forced.is_valid_int():
		return clampi(int(forced), 1, BiomeParents.COUNT - 1)
	if province_id != "" and DataFile.exists(PROVINCES_FILE):
		var table: Variant = DataFile.load_cached(PROVINCES_FILE)
		if table is Dictionary:
			var biome := int(((table as Dictionary).get("provinces", {}) as Dictionary).get(province_id, 0))
			if biome > 0:
				return biome
	return DEFAULT_BIOME


static func _ensure() -> void:
	if _loaded:
		return
	_loaded = true
	if not TextureQuality.use_tx() or not DataFile.exists(PACK_FILE):
		return
	var parsed: Variant = DataFile.read_json(PACK_FILE)
	if parsed is Dictionary:
		_pack = parsed


## Groupe ("green", "dry", "steppe", "alpine", "arctic") qui sert `biome` (repli sur le parent), "" sans paquet.
static func group_of(biome: int) -> String:
	_ensure()
	var cards: Array = _pack.get("cards", [])
	if cards.is_empty():
		return ""
	var serves := func(b: int) -> bool:
		for card: Dictionary in cards:
			if _lists(card, b):
				return true
		return false
	var resolved := BiomeParents.resolve(biome, serves)
	for card: Dictionary in cards:
		if _lists(card, resolved):
			return str(card["id"]).get_slice("_", 2) if str(card["id"]).count("_") >= 2 else ""
	return ""


## `biomes` d'une carte lu du JSON (flottants) : comparaison entière.
static func _lists(card: Dictionary, biome: int) -> bool:
	for value: Variant in card.get("biomes", []):
		if int(value) == biome:
			return true
	return false


## Cartes du groupe de `biome` : {rôle: {"file", "luma"}} (rôles `grass_blades`, `grass_tufts`,
## `grass_clump`) ; {} sans paquet.
static func cards(biome: int) -> Dictionary:
	var group := group_of(biome)
	if group == "":
		return {}
	var out := {"group": group}
	for card: Dictionary in _pack.get("cards", []):
		if str(card["id"]).ends_with("_" + group):
			out[str(card["role"])] = {"file": str(card["file"]), "luma": float(card["luma"])}
	return out


## Carte `role` du jeu `set` (résultat de `cards`), ou `fallback`.
static func texture(set: Dictionary, role: String, fallback: Texture2D) -> Texture2D:
	var entry: Variant = set.get(role)
	if entry is Dictionary and ResourceLoader.exists(str(entry["file"])):
		return load(str(entry["file"])) as Texture2D
	return fallback


## Luminance moyenne (linéaire) de la carte `role` du jeu `set`, `fallback` sans carte.
static func luma(set: Dictionary, role: String, fallback: float) -> float:
	var entry: Variant = set.get(role)
	return float((entry as Dictionary)["luma"]) if entry is Dictionary else fallback


## FA7 hors groupe vert : atlas de l'herbe avec les cartes du groupe dans les cases d'herbe haute et
## folle ; pose `grass_texture` et `tex_lum` sur `mat`. Le groupe vert garde l'atlas d'avant.
static func apply_fa(mat: ShaderMaterial, set: Dictionary, fa: Dictionary, atlas_path: String) -> void:
	var group := str(set.get("group", ""))
	if group == "" or group == "green":
		return
	if not _fa_cache.has(group):
		_fa_cache[group] = _compose_fa(set, fa, atlas_path)
	var composed: Dictionary = _fa_cache[group]
	if composed.is_empty():
		return
	mat.set_shader_parameter("grass_texture", composed["texture"])
	mat.set_shader_parameter("tex_lum", composed["luma"])


static func _image_of(path: String) -> Image:
	var texture := load(path) as Texture2D
	if texture == null:
		return null
	var image := texture.get_image()
	if image == null:
		return null
	if image.is_compressed():
		image.decompress()
	image.convert(Image.FORMAT_RGBA8)
	return image


static func _compose_fa(set: Dictionary, fa: Dictionary, atlas_path: String) -> Dictionary:
	var atlas := _image_of(atlas_path)
	if atlas == null:
		return {}
	var spec: Dictionary = fa["atlas"]
	var cell := Vector2i(int(spec["cell_width"]), int(spec["cell_height"]))
	var columns := int(spec["columns"])
	var pad := int(spec["padding"])
	var variants: Array = fa["variants"]
	var luma_sum := 0.0
	var luma_count := 0
	var base_luma := float((fa["render"] as Dictionary).get("tex_lum", 0.2))
	var cards_by_role: Dictionary = {}
	for index in variants.size():
		var role: String = FA_CELLS.get(str(variants[index]["name"]), "")
		if role == "" or not set.has(role):
			continue
		if not cards_by_role.has(role):
			cards_by_role[role] = _image_of(str((set[role] as Dictionary)["file"]))
		var card: Image = cards_by_role[role]
		if card == null:
			continue
		var side := cell.y - 2 * pad
		var scaled := card.duplicate() as Image
		scaled.resize(side, side, Image.INTERPOLATE_LANCZOS)
		var origin := Vector2i((index % columns) * cell.x, (index / columns) * cell.y)
		atlas.fill_rect(Rect2i(origin, cell), Color(0, 0, 0, 0))
		atlas.blit_rect(scaled, Rect2i(Vector2i.ZERO, scaled.get_size()), origin + Vector2i((cell.x - side) / 2, pad))
		luma_sum += float((set[role] as Dictionary)["luma"])
		luma_count += 1
	if luma_count == 0:
		return {}
	atlas.generate_mipmaps()
	# Cases conservées (chaume, blé) à la luminance de l'atlas d'avant, cases remplacées à celle de leur carte.
	var mean_luma := (luma_sum + float(variants.size() - luma_count) * base_luma) / float(variants.size())
	return {"texture": ImageTexture.create_from_image(atlas), "luma": mean_luma}
