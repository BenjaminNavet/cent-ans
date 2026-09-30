class_name BattleSkinned
extends RefCounted

## Figurines skinnées (lot V2) : chargement des maillages et des textures d'os cuits par
## `tools/blender_scripts/battle_skinned.py` (`assets/models/battle_skinned/`), matériau
## `battle_soldier_skinned.gdshader` et correspondance état du régiment → clips.
## Repli : sans manifeste, ou avec `--rigid-figures` / `--legacy-figures` après `--`, les
## figurines à membres rigides (lots B1/B4, `BattleMeshes`) restent utilisées.
## Lot FG1 : les figurines fines (corps MakeHuman, rigs aux proportions réalistes) de
## `assets/models/battle_fine/` remplacent celles qu'elles couvrent (rigs renommés `fine_human` /
## `fine_cavalry`, chemins absolus dans le manifeste fusionné). Lot FG5 (ADR 0089) : rendu par
## défaut ; `--coarse-figures` après `--` rend les figurines Quaternius (V2) le temps de la
## transition, `--no-fg3` les figurines fines sans cartes cuites.

const DIR := "res://assets/models/battle_skinned/"
const FINE_DIR := "res://assets/models/battle_fine/"
const FINE_RIG_PREFIX := "fine_"
const SHADER := preload("res://shaders/battle_soldier_skinned.gdshader")
const MAX_CLIPS := 96  # AN1b : 48 -> 64 ; NT7 : 96 (clips[] des shaders)
## Modes du shader.
const M_LOOP := 0
const M_CYCLE := 1
const M_VOLLEY := 2
const M_CUSTOM := 3
const M_SPLIT := 4  # SG1 : escalade (premiers soldats sur les échelles, le reste au pied du mur)
## AN1b : taille maximale d'un jeu de clips (deux indices par composante de `clip_set`).
const MAX_SET := 8

static var _manifest: Dictionary = {}
static var _loaded: bool = false
static var _meshes: Dictionary = {}
static var _textures: Dictionary = {}
static var _configs: Dictionary = {}  # "kind/variant/state" -> configuration (chaque image)
## NT7 : réglages d'animation (`data/fx/battle_animation.json`).
const ANIMATION_FILE := "fx/battle_animation.json"
static var _animation: Dictionary = {}


## NT7 : réglages d'animation (fondu de cycle, clips de rôle), lus une fois.
static func animation_settings() -> Dictionary:
	if _animation.is_empty():
		_animation = BattleStandards.read_data(ANIMATION_FILE)
	return _animation


## NT7 : durée (s) du fondu entre deux clips d'un cycle de mêlée ; 0 avec `--no-nt7` après `--`
## (banc A/B : changement sec comme avant).
static func cycle_blend_s() -> float:
	if "--no-nt7" in OS.get_cmdline_user_args():
		return 0.0
	return float(animation_settings().get("cycle_blend_s", 0.0))


## NT7 : miroir GDScript du tirage du mode CYCLE et du fondu du shader (tests, captures) : clip
## courant, clip du cycle précédent et poids du courant (1 = pas de fondu) pour le soldat de
## hachages `h` à l'instant `anim_time`. Même arithmétique que `choose`/`cycle_prev` (le sinus du
## hachage peut différer du GPU au dernier bit : sert à vérifier le principe, pas le pixel).
static func cycle_blend_at(config: Dictionary, h: Vector4, anim_time: float, blend: float) -> Dictionary:
	var ids: Array = config["set"]
	var n := maxi(ids.size(), 1)
	var cycle := float(config.get("cycle", 1.5))
	var local := anim_time * lerpf(0.9, 1.1, h.y) + h.z * cycle
	var cyc := floorf(local / cycle)
	var t := fposmod(local, cycle)
	var clip: int = ids[mini(int(_hash1(h.x * 91.7 + cyc * 13.1) * n), n - 1)]
	var result := {"clip": clip, "prev": clip, "t": t, "prev_t": 0.0, "weight": 1.0}
	if blend <= 0.001 or n < 2 or t >= blend:
		return result
	var prev: int = ids[mini(int(_hash1(h.x * 91.7 + (cyc - 1.0) * 13.1) * n), n - 1)]
	if prev == clip:
		return result
	result["prev"] = prev
	result["prev_t"] = cycle + t
	result["weight"] = smoothstep(0.0, blend, t)
	return result


## NT10 : durée (s) du fondu au changement de clip des figurines en mode CUSTOM (porte-étendards,
## musiciens, servants d'engins) ; 0 avec `--no-nt10` après `--` (banc A/B, changement sec).
static func role_blend_s() -> float:
	if "--no-nt10" in OS.get_cmdline_user_args():
		return 0.0
	return float(animation_settings().get("role_blend_s", 0.0))


## NT10 : suit le clip (indice global dans `clips[]`) d'une figurine en mode CUSTOM ; `state` est
## un dictionnaire propre à la figurine (modifié : `cur`, `prev`, `at`). Renvoie le clip précédent
## encore en fondu, ou −1 (le fondu passé, ou horloge revenue en arrière).
static func fade_prev(state: Dictionary, clip: int, now: float) -> int:
	var cur := int(state.get("cur", -1))
	if cur != clip:
		if cur >= 0:
			state["prev"] = cur
			state["at"] = now
		state["cur"] = clip
	var prev := int(state.get("prev", -1))
	if prev >= 0:
		var since := now - float(state.get("at", now))
		if since < 0.0 or since > role_blend_s() + 0.1 or role_blend_s() <= 0.0:
			state["prev"] = -1
			prev = -1
	return prev


## NT10 : INSTANCE_CUSTOM.y du mode CUSTOM avec fondu (`custom_fade` du shader) : emplacement du
## clip dans le jeu (0-31) + 32 × (clip précédent + 1) + 4096 × q, q = instant du changement en
## 1/32 s modulo 64 s (arrondi par défaut : le fondu ne part jamais en avance). Entier exact en
## flottant 32 bits (< 2^24).
static func pack_fade(slot: int, prev_clip: int, changed_at: float) -> float:
	if prev_clip < 0:
		return float(slot)
	var q := int(floorf(fposmod(changed_at, 64.0) * 32.0)) % 2048
	return float((slot & 31) + 32 * (clampi(prev_clip, 0, 126) + 1) + 4096 * q)


static func _hash1(n: float) -> float:
	var x := sin(n * 12.9898 + 4.1414) * 43758.5453
	return x - floorf(x)


## Lot BV2 : variante « cadavres » du shader (`BV2_CORPSE` : coupe des parties tranchées par
## `discard`, réservée aux cadavres pour ne pas pénaliser les soldats vivants).
static var _corpse_shader: Shader = null


static func corpse_shader() -> Shader:
	# FG3 : avec les figurines fines, les cadavres gardent les cartes cuites (même variante,
	# uniformes `fine_*` à leur valeur neutre pour une figurine sans atlas).
	if fine_enabled() and fine_maps_ready():
		return _variant(["BV2_CORPSE", "FG3_BAKED"])
	if _corpse_shader == null:
		_corpse_shader = _variant(["BV2_CORPSE"])
	return _corpse_shader


static var _variants: Dictionary = {}


## Variante du shader skinné avec les `defines` en tête (après `shader_type`).
static func _variant(defines: Array) -> Shader:
	var key := ",".join(defines)
	if _variants.has(key):
		return _variants[key]
	var code := SHADER.code
	var cut := code.find("\n", code.find("shader_type"))
	var head := ""
	for d in defines:
		head += "#define %s\n" % d
	var shader := Shader.new()
	shader.code = code.substr(0, cut + 1) + head + code.substr(cut + 1)
	_variants[key] = shader
	return shader


## Lot FG3 : cartes cuites des figurines fines (`assets/models/battle_fine/textures/`).
## `lod0` / `lod1` : atlas par figurine (Texture2DArray, couche = `atlas_layer` du manifeste ;
## RG normale de forme, B occlusion, A masque selon la matière) ; `detail` : tuiles partagées
## par matière (Texture2DArray) ; `horse` : pelage CC0 réduit (normale, relief, occlusion).
const FINE_TEX_DIR := FINE_DIR + "textures/"
const FINE_MAPS := {
	"lod0": "fine_atlas_lod0.png",
	"lod1": "fine_atlas_lod1.png",
	"detail": "fine_detail.png",
	"horse": "fine_horse.png",
}
## Lot GA1 (ADR 0104) : matières générées. `detail` prend `fine_detail_ga1.png` (12 couches,
## `data/art/materials.yaml`) et `detail_albedo` s'ajoute (albédo de détail centré, multiplié à
## la couleur de sommet). `--no-ga1` après `--` : tuiles FG3 d'origine (mesures A/B) ; l'ancien
## tableau n'est alors chargé qu'à la place du nouveau (pas de double mémoire).
const GA1_DETAIL := "fine_detail_ga1.png"
const GA1_ALBEDO := "fine_detail_albedo.png"
static var _fine_maps: Dictionary = {}
static var _fine_maps_loaded := false


static func fine_maps() -> Dictionary:
	if not _fine_maps_loaded:
		_fine_maps_loaded = true
		# `--no-fg3` : figurines fines sans cartes cuites (mesures A/B).
		if OS.get_cmdline_user_args().has("--no-fg3"):
			return _fine_maps
		var ga1 := (
			not OS.get_cmdline_user_args().has("--no-ga1")
			and ResourceLoader.exists(FINE_TEX_DIR + GA1_DETAIL)
			and ResourceLoader.exists(FINE_TEX_DIR + GA1_ALBEDO)
		)
		for key in FINE_MAPS:
			var file := GA1_DETAIL if ga1 and key == "detail" else str(FINE_MAPS[key])
			var path: String = FINE_TEX_DIR + file
			if ResourceLoader.exists(path):
				_fine_maps[key] = load(path)
		if ga1:
			_fine_maps["detail_albedo"] = load(FINE_TEX_DIR + GA1_ALBEDO)
		var complete := true
		for key in FINE_MAPS:
			complete = complete and _fine_maps.has(key)
		if not complete:
			if fine_enabled():
				push_warning("BattleSkinned: cartes FG3 incomplètes dans %s" % FINE_TEX_DIR)
			_fine_maps = {}
	return _fine_maps


static func fine_maps_ready() -> bool:
	return not fine_maps().is_empty()


## Lot SR2 : usure des figurines fines cuites (0-1). `--no-sr2` après `--` : 0 (rendu SR1 exact).
const SR2_WEATHERING := 0.85
const SR2_MUD_HEIGHT := 0.45
const SR2_MUD_HEIGHT_HORSE := 0.65


static func sr2_weathering() -> float:
	return 0.0 if OS.get_cmdline_user_args().has("--no-sr2") else SR2_WEATHERING


## GA1 : albédo de détail généré actif (absent avec `--no-ga1` ou sans les tableaux).
static func ga1_enabled() -> bool:
	return fine_maps().has("detail_albedo")


## FG3 : bascule le matériau d'une figurine fine cuite sur la variante `FG3_BAKED` et pose
## ses cartes. Sans effet pour les autres figurines (rendu par défaut inchangé).
static func _setup_fine_maps(mat: ShaderMaterial, kind: String, variant: int) -> void:
	var fig := figure(kind, variant)
	if not fig.has("atlas_layer") or not fine_maps_ready():
		return
	# Matériaux d'un autre shader qui lisent la texture d'os (drapeau porté d'EP5, etc.) :
	# garder leur shader, sans cartes.
	if mat.shader != SHADER and not _variants.values().has(mat.shader):
		return
	var corpse := mat.shader != null and mat.shader.code.contains("#define BV2_CORPSE")
	mat.shader = _variant(["BV2_CORPSE", "FG3_BAKED"] if corpse else ["FG3_BAKED"])
	var maps := fine_maps()
	mat.set_shader_parameter("fine_atlas0", maps["lod0"])
	mat.set_shader_parameter("fine_atlas1", maps["lod1"])
	mat.set_shader_parameter("fine_detail", maps["detail"])
	mat.set_shader_parameter("ga1_detail", maps.has("detail_albedo"))
	if maps.has("detail_albedo"):
		mat.set_shader_parameter("fine_detail_albedo", maps["detail_albedo"])
	mat.set_shader_parameter("fine_horse", maps["horse"])
	mat.set_shader_parameter("fine_layer", int(fig["atlas_layer"]))
	# SR2 : usure (boue des pieds, crasse des creux, acier vivant, teintes passées) ; les
	# cavaliers salissent le bas des jambes du cheval et l'ourlet du caparaçon (~0,65 m).
	mat.set_shader_parameter("weathering", sr2_weathering())
	mat.set_shader_parameter("sr2_mud_height", SR2_MUD_HEIGHT_HORSE if kind == "cavalry" else SR2_MUD_HEIGHT)


static func manifest() -> Dictionary:
	if not _loaded:
		_loaded = true
		var text := FileAccess.get_file_as_string(DIR + "manifest.json")
		var parsed = JSON.parse_string(text) if text != "" else null
		_manifest = parsed if parsed is Dictionary else {}
		if fine_enabled() and not _manifest.is_empty():
			_merge_fine(_manifest)
	return _manifest


## Lot FG1 : figurines fines actives. FG5 : par défaut ; `--coarse-figures` après `--` les
## coupe (figurines Quaternius du lot V2). `--fine-figures` reste accepté (sans effet).
static func fine_enabled() -> bool:
	return not OS.get_cmdline_user_args().has("--coarse-figures")


## FG5 : figurine fine (LOD0 dessiné par soldat, voir `BattleSoldiers.FINE_DETAIL_DISTANCE`).
static func is_fine(kind: String, variant: int) -> bool:
	return bool(figure(kind, variant).get("fine", false))


## Lot FG1 : ajoute au manifeste les rigs fins (renommés) et remplace les figurines fines.
static func _merge_fine(base: Dictionary) -> void:
	var text := FileAccess.get_file_as_string(FINE_DIR + "manifest.json")
	var parsed = JSON.parse_string(text) if text != "" else null
	if not parsed is Dictionary:
		push_warning("BattleSkinned: figurines fines sans manifeste %s" % FINE_DIR)
		return
	var rigs: Dictionary = base.get("rigs", {})
	for rig_name in (parsed as Dictionary).get("rigs", {}):
		var entry: Dictionary = (parsed["rigs"][rig_name] as Dictionary).duplicate(true)
		entry["texture"] = FINE_DIR + str(entry.get("texture", ""))
		rigs[FINE_RIG_PREFIX + str(rig_name)] = entry
	var figures: Dictionary = base.get("figures", {})
	for fig_name in (parsed as Dictionary).get("figures", {}):
		var entry: Dictionary = (parsed["figures"][fig_name] as Dictionary).duplicate(true)
		var lods: Array = []
		for file in entry.get("lods", []):
			lods.append(FINE_DIR + str(file))
		entry["lods"] = lods
		entry["rig"] = FINE_RIG_PREFIX + str(entry.get("rig", ""))
		figures[fig_name] = entry
	base["rigs"] = rigs
	base["figures"] = figures


## Chemin d'un binaire du manifeste (relatif à `DIR`, ou absolu pour les figurines fines).
static func _path(file: String) -> String:
	return file if file.begins_with("res://") else DIR + file


static func enabled() -> bool:
	var args := OS.get_cmdline_user_args()
	return not args.has("--rigid-figures") and not args.has("--legacy-figures")


static func figure_name(kind: String, variant: int) -> String:
	return "%s_%d" % [kind, variant]


## Vrai si la figurine skinnée de `kind`/`variant` existe (et que le rendu skinné est actif).
static func has_figure(kind: String, variant: int) -> bool:
	if not enabled():
		return false
	var figures: Dictionary = manifest().get("figures", {})
	return figures.has(figure_name(kind, variant))


static func figure(kind: String, variant: int) -> Dictionary:
	return manifest().get("figures", {}).get(figure_name(kind, variant), {})


static func rig(kind: String, variant: int) -> Dictionary:
	return manifest().get("rigs", {}).get(str(figure(kind, variant).get("rig", "")), {})


## Maillage de la figurine au niveau `level` (0 complet, 1 moyen, 2 lointain/ombres).
static func mesh(kind: String, variant: int, level: int) -> ArrayMesh:
	var fig := figure(kind, variant)
	var lods: Array = fig.get("lods", [])
	if lods.is_empty():
		return null
	var file: String = lods[clampi(level, 0, lods.size() - 1)]
	if _meshes.has(file):
		return _meshes[file]
	var mesh := _load_mesh(_path(file), not str(fig.get("rig", "")).ends_with("human"))
	_meshes[file] = mesh
	return mesh


## Maillage binaire `CAM1` (zlib) : positions, normales, couleurs (rgb + code), UV, os, poids,
## masques de variante, indices. `CAM2` (lot FG3) : en plus, après les masques, l'UV d'atlas
## empaquetée des cartes cuites (u, v sur 11 bits, source sur 2 bits).
static func _load_mesh(path: String, large: bool) -> ArrayMesh:
	var bytes := FileAccess.get_file_as_bytes(path)
	var magic := bytes.slice(0, 4).get_string_from_ascii() if bytes.size() >= 16 else ""
	if magic != "CAM1" and magic != "CAM2":
		push_warning("BattleSkinned: bad mesh file %s" % path)
		return null
	# FG3 : `CAM2` = `CAM1` + une UV d'atlas empaquetée par sommet (UV2.y, cf. le shader).
	var stride := 22 if magic == "CAM2" else 21
	var n := bytes.decode_u32(4)
	var m := bytes.decode_u32(8)
	var raw := bytes.slice(16).decompress(bytes.decode_u32(12), FileAccess.COMPRESSION_DEFLATE)
	var floats := raw.slice(0, n * stride * 4).to_float32_array()
	var indices := raw.slice(n * stride * 4, n * stride * 4 + m * 4).to_int32_array()
	var verts := PackedVector3Array()
	var normals := PackedVector3Array()
	var colors := PackedColorArray()
	var uvs := PackedVector2Array()
	var uv2 := PackedVector2Array()
	verts.resize(n)
	normals.resize(n)
	colors.resize(n)
	uvs.resize(n)
	uv2.resize(n)
	var o_n := n * 3
	var o_c := n * 6
	var o_uv := n * 10
	var o_b := n * 12
	var o_w := n * 16
	var o_m := n * 20
	var o_a := n * 21
	for i in n:
		verts[i] = Vector3(floats[i * 3], floats[i * 3 + 1], floats[i * 3 + 2])
		normals[i] = Vector3(floats[o_n + i * 3], floats[o_n + i * 3 + 1], floats[o_n + i * 3 + 2])
		# Couleurs stockées en 8 bits : le code matière passe en alpha / 16.
		colors[i] = Color(floats[o_c + i * 4], floats[o_c + i * 4 + 1], floats[o_c + i * 4 + 2], floats[o_c + i * 4 + 3] / 16.0)
		uvs[i] = Vector2(floats[o_uv + i * 2], floats[o_uv + i * 2 + 1])
		uv2[i] = Vector2(floats[o_m + i], floats[o_a + i] if stride == 22 else 0.0)
	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = verts
	arrays[Mesh.ARRAY_NORMAL] = normals
	arrays[Mesh.ARRAY_COLOR] = colors
	arrays[Mesh.ARRAY_TEX_UV] = uvs
	arrays[Mesh.ARRAY_TEX_UV2] = uv2
	arrays[Mesh.ARRAY_CUSTOM0] = floats.slice(o_b, o_b + n * 4)
	arrays[Mesh.ARRAY_CUSTOM1] = floats.slice(o_w, o_w + n * 4)
	arrays[Mesh.ARRAY_INDEX] = indices
	var mesh := ArrayMesh.new()
	var flags := (Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM0_SHIFT) | (Mesh.ARRAY_CUSTOM_RGBA_FLOAT << Mesh.ARRAY_FORMAT_CUSTOM1_SHIFT)
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays, [], {}, flags)
	# Boîte englobante élargie : les animations (coups, chutes) sortent de la pose de repos.
	mesh.custom_aabb = AABB(Vector3(-2.5, -0.5, -3.0), Vector3(5.0, 4.5, 6.5)) if large else AABB(Vector3(-1.6, -0.3, -1.8), Vector3(3.2, 2.8, 3.8))
	return mesh


## Texture d'os d'un rig (`CAB1` : os, images, puis zlib de 3 texels RGBA32F par os et image).
static func bone_texture(rig_name: String) -> ImageTexture:
	if _textures.has(rig_name):
		return _textures[rig_name]
	var entry: Dictionary = manifest().get("rigs", {}).get(rig_name, {})
	var bytes := FileAccess.get_file_as_bytes(_path(str(entry.get("texture", ""))))
	var tex: ImageTexture = null
	if bytes.size() > 12 and bytes.slice(0, 4).get_string_from_ascii() == "CAB1":
		var bones := bytes.decode_u32(4)
		var frames := bytes.decode_u32(8)
		var raw := bytes.slice(12).decompress(bones * 3 * frames * 16, FileAccess.COMPRESSION_DEFLATE)
		var image := Image.create_from_data(bones * 3, frames, false, Image.FORMAT_RGBAF, raw)
		tex = ImageTexture.create_from_image(image)
	else:
		push_warning("BattleSkinned: missing bone texture for rig %s" % rig_name)
	_textures[rig_name] = tex
	return tex


static func clip_index(rig_entry: Dictionary, clip: String) -> int:
	var names: Array = rig_entry.get("clips", {}).keys()
	names.sort()
	return maxi(names.find(clip), 0)


## Uniformes propres au rig et à la figurine (texture, table des clips, variantes).
static func setup_material(mat: ShaderMaterial, kind: String, variant: int) -> void:
	var rig_entry := rig(kind, variant)
	mat.set_shader_parameter("bone_tex", bone_texture(str(figure(kind, variant).get("rig", ""))))
	mat.set_shader_parameter("bone_fps", float(rig_entry.get("fps", 24)))
	var table: Array[Vector4] = []
	var clips: Dictionary = rig_entry.get("clips", {})
	var names: Array = clips.keys()
	names.sort()
	for name in names:
		var c: Dictionary = clips[name]
		table.append(Vector4(float(c["start"]), float(c["frames"]), 1.0 if bool(c["loop"]) else 0.0, 0.0))
	while table.size() < MAX_CLIPS:
		table.append(Vector4(0, 1, 1, 0))
	mat.set_shader_parameter("clips", table)
	mat.set_shader_parameter("cycle_blend", cycle_blend_s())
	mat.set_shader_parameter("role_blend", role_blend_s())
	mat.set_shader_parameter("variant_count", int(figure(kind, variant).get("variants", 1)))
	mat.set_shader_parameter("size_jitter", 0.0 if kind == "cavalry" else 0.05)
	mat.set_shader_parameter("sever_bones", sever_table(kind, variant))
	_setup_fine_maps(mat, kind, variant)


## Configuration d'animation {set: [clips], mode, speed, cycle} d'un régiment dans l'état
## `state` (clé de la simulation : idle, marching, charging, melee, shooting, routing,
## climbing ; `running` = marche au pas de course ; états de rendu : brace, victory,
## melee_pikes). AN1b : les clips absents du rig (kit grossier antérieur) sont écartés du jeu ;
## jeu vide -> `fallback`, sinon le jeu `idle`.
static func state_config(kind: String, variant: int, state: String, running: bool) -> Dictionary:
	var key := state
	if state == "marching" and running:
		key = "running"
	var cache_key := "%s/%d/%s" % [kind, variant, key]
	if _configs.has(cache_key):
		return _configs[cache_key]
	var sets: Dictionary = STYLES.get(_style(kind, variant), STYLES["sword"])
	if not sets.has(key):
		key = "idle"
	var entry: Dictionary = sets[key]
	var rig_entry := rig(kind, variant)
	var names := _present(rig_entry, entry["set"])
	# EP12 : jeu de repli quand le manifeste n'a pas encore les clips (kit antérieur).
	if entry.has("fallback") and (names.is_empty() or str(names[0]) != str((entry["set"] as Array)[0])):
		names = entry["fallback"]
	if names.is_empty():
		entry = sets["idle"]
		names = _present(rig_entry, entry["set"])
	if names.is_empty():
		names = [str((entry["set"] as Array)[0])]
	var ids: Array[int] = []
	for c in names:
		ids.append(clip_index(rig_entry, str(c)))
	var config := {"key": cache_key, "names": names, "set": ids, "mode": int(entry.get("mode", M_LOOP)), "speed": float(entry.get("speed", 1.0)), "cycle": float(entry.get("cycle", 1.5)), "release": float(entry.get("release", 1.0))}
	_configs[cache_key] = config
	return config


## Clips de `names` présents dans le rig (ordre et doublons gardés : les doublons pèsent dans
## le tirage), au plus `MAX_SET`.
static func _present(rig_entry: Dictionary, names: Array) -> Array:
	var clips: Dictionary = rig_entry.get("clips", {})
	var out: Array = []
	for c in names:
		if clips.has(str(c)) and out.size() < MAX_SET:
			out.append(str(c))
	return out


## Clips de mort (mode CUSTOM des cadavres, INSTANCE_CUSTOM.y = indice dans ce jeu).
static func death_config(kind: String, variant: int) -> Dictionary:
	var rig_entry := rig(kind, variant)
	var names: Array = DEATHS_CAVALRY if kind == "cavalry" else DEATHS_FOOT
	var ids: Array[int] = []
	for c in names:
		if rig_entry.get("clips", {}).has(c):
			ids.append(clip_index(rig_entry, str(c)))
	if ids.is_empty():
		ids.append(0)
	return {"key": "%s/%d/dead" % [kind, variant], "set": ids, "mode": M_CUSTOM, "speed": 1.0, "cycle": 1.0, "release": 1.0}


## Lot EP12 : blessés au sol (mode CUSTOM, INSTANCE_CUSTOM.y = indice dans ce jeu) ; vide si
## le rig n'a pas les clips (kit antérieur) ou pour les cavaliers.
static func wounded_config(kind: String, variant: int) -> Dictionary:
	if kind == "cavalry":
		return {}
	var rig_entry := rig(kind, variant)
	var ids: Array[int] = []
	for c in WOUNDED_FOOT:
		if not rig_entry.get("clips", {}).has(c):
			return {}
		ids.append(clip_index(rig_entry, str(c)))
	return {"key": "%s/%d/wounded" % [kind, variant], "set": ids, "mode": M_CUSTOM, "speed": 1.0, "cycle": 1.0, "release": 1.0}


## Style d'animation de la figurine (`sword`, `pike`, `bow`...), exposé au rendu (EP12 : armes
## laissées au sol par les fuyards).
static func style_of(kind: String, variant: int) -> String:
	return _style(kind, variant)


## Style d'animation de la figurine : champ `style` du manifeste (lot UR1), sinon règle
## historique par famille et variante.
static func _style(kind: String, variant: int) -> String:
	var style := str(figure(kind, variant).get("style", ""))
	if STYLES.has(style):
		return style
	match kind:
		"infantry":
			return "sword" if variant == 0 else "pike" if variant == 1 else "militia"
		"archer":
			return "bow" if variant == 0 else "crossbow"
		"cavalry":
			return "horse_bow" if variant == 2 else "lance"
	return "sword"


## Figurine « noble » (livrée plus présente) : champ `noble` du manifeste (lot UR1), sinon
## variante 0 des fantassins et des cavaliers.
## DA1 : boîte du buste en pose de repos, où le shader peint les armoiries (surcot, jaque) ou la
## croix de livrée : sommets livrée (code 0) dont l'os dominant est le torse ou la poitrine.
## {center: Vector2(x, y), size: Vector2(largeur, hauteur), depth: Vector2(z min, z max)} ;
## vide si la figurine n'a pas de livrée au buste (plates complètes).
static var _chest_boxes: Dictionary = {}


static func chest_box(kind: String, variant: int) -> Dictionary:
	var key := figure_name(kind, variant)
	if _chest_boxes.has(key):
		return _chest_boxes[key]
	var result := {}
	var mesh_lod := mesh(kind, variant, 0)
	var bones: Array = rig(kind, variant).get("bones", [])
	var torso := {}
	for index in bones.size():
		var bone_name := str(bones[index]).trim_prefix("R:")
		if bone_name == "Torso" or bone_name == "Chest":
			torso[index] = true
	if mesh_lod != null and not torso.is_empty():
		var arrays := mesh_lod.surface_get_arrays(0)
		var verts: PackedVector3Array = arrays[Mesh.ARRAY_VERTEX]
		var colors: PackedColorArray = arrays[Mesh.ARRAY_COLOR]
		var bone_idx: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM0]
		var weights: PackedFloat32Array = arrays[Mesh.ARRAY_CUSTOM1]
		var low := Vector3(INF, INF, INF)
		var high := -low
		for i in verts.size():
			if int(colors[i].a * 16.0 + 0.5) != 0:
				continue
			var best := 0
			for k in range(1, 4):
				if weights[i * 4 + k] > weights[i * 4 + best]:
					best = k
			if not torso.has(int(bone_idx[i * 4 + best] + 0.5)):
				continue
			low = low.min(verts[i])
			high = high.max(verts[i])
		if low.x < high.x:
			var width := (high.x - low.x) + 0.06
			result = {
				"center": Vector2((low.x + high.x) * 0.5, (low.y + high.y) * 0.5),
				"size": Vector2(width, width * 1.25),
				"depth": Vector2(low.z - 0.06, high.z + 0.06),
			}
	_chest_boxes[key] = result
	return result


static func is_noble(kind: String, variant: int) -> bool:
	var fig := figure(kind, variant)
	if fig.has("noble"):
		return bool(fig["noble"])
	return variant == 0 and (kind == "infantry" or kind == "cavalry")


## Variante des figurines rigides (B1/B4, trois par famille) la plus proche d'une figurine
## skinnée de variante quelconque (lot UR1), d'après son style d'animation.
static func rigid_variant(kind: String, variant: int) -> int:
	if variant <= 2 or kind == "siege":
		return variant
	match _style(kind, variant):
		"sword":
			return 0
		"pike":
			return 1
		"militia":
			return 2
		"bow":
			return 0
		"crossbow":
			return 1
		"horse_bow", "horse_javelin":
			return 2
		"lance":
			return 0 if is_noble(kind, variant) else 1
	return 0


const DEATHS_FOOT := ["death", "death_m", "death_back", "death_knees"]
## Lot EP12 : blessés (rampe, assis, à genoux) ; ordre = INSTANCE_CUSTOM.y de la couche.
const WOUNDED_FOOT := ["crawl", "wounded_sit", "wounded_kneel"]
## Lot EP12 : drapeau ajouté au code de INSTANCE_CUSTOM.w (cadavre ou blessé désarmé).
const CODE_UNARMED := 8
## Lot BV2 : `c_fall` = cavalier désarçonné (le cheval s'enfuit, code 6 du shader).
const DEATHS_CAVALRY := ["c_death", "c_death_m", "c_fall"]
## Lot BV2 : parties tranchées (code de INSTANCE_CUSTOM.w, 1-5) → os du rig (plage, plus deux
## os isolés : l'arme suit la main). Entrée 0 : os du cheval (tout ce qui n'est pas `R:`).
const SEVER_PARTS := {
	"head": [1, ["Head"]],
	"arm_r": [2, ["LowerArm.R", "Wrist.R", "Prop"]],
	"arm_l": [3, ["LowerArm.L", "Wrist.L"]],
	"leg_r": [4, ["LowerLeg.R", "Foot.R"]],
	"leg_l": [5, ["LowerLeg.L", "Foot.L"]],
}
const CODE_HORSE_FLEES := 6


## Indice de `clip` dans le jeu des morts de la figurine (-1 : absent).
static func death_index(kind: String, variant: int, clip: String) -> int:
	var names: Array = DEATHS_CAVALRY if kind == "cavalry" else DEATHS_FOOT
	var clips: Dictionary = rig(kind, variant).get("clips", {})
	var k := 0
	for c in names:
		if clips.has(c):
			if c == clip:
				return k
			k += 1
	return -1


## Durée (s) d'un clip du rig de la figurine.
static func clip_seconds(kind: String, variant: int, clip: String) -> float:
	var entry := rig(kind, variant)
	var c: Dictionary = entry.get("clips", {}).get(clip, {})
	return float(c.get("frames", 24)) / float(entry.get("fps", 24))


## Renversés (lot BV2) : clip `knockdown` en mode CUSTOM (à pied seulement).
static func knockdown_config(kind: String, variant: int) -> Dictionary:
	var rig_entry := rig(kind, variant)
	var ids: Array[int] = [clip_index(rig_entry, "knockdown")]
	return {"key": "%s/%d/knock" % [kind, variant], "set": ids, "mode": M_CUSTOM, "speed": 1.0, "cycle": 1.0, "release": 1.0}


## Table `sever_bones` du shader (lot BV2) pour le rig de la figurine.
static func sever_table(kind: String, variant: int) -> Array[Vector4i]:
	var bones: Array = rig(kind, variant).get("bones", [])
	var prefix := "R:" if kind == "cavalry" else ""
	var table: Array[Vector4i] = []
	for i in 6:
		table.append(Vector4i(-1, -2, -1, -1))
	if kind == "cavalry":
		var last := -1
		for i in bones.size():
			if not str(bones[i]).begins_with("R:"):
				last = i
		table[0] = Vector4i(0, last, -1, -1)
	for part in SEVER_PARTS:
		var entry: Array = SEVER_PARTS[part]
		var ids: Array[int] = []
		for name in entry[1]:
			var k := bones.find(prefix + str(name))
			if k >= 0:
				ids.append(k)
		if ids.is_empty():
			continue
		# Os consécutifs en plage (x..y), le dernier (arme) en z s'il ne suit pas.
		var lo: int = ids[0]
		var hi: int = lo
		var extra := -1
		for k in range(1, ids.size()):
			if ids[k] == hi + 1:
				hi = ids[k]
			else:
				extra = ids[k]
		table[int(entry[0])] = Vector4i(lo, hi, extra, -1)
	return table

## AN1b : charge des lanciers en cycles de six foulées de galop (90 images, 3,75 s) : un cycle
## sur huit est un trébuchement (`c_stumble`, même galop avant et après) ; sans ce clip (kit
## grossier), le jeu se réduit à `c_charge` et le galop reste continu d'un cycle à l'autre.
const CAVALRY_CHARGE := {"set": ["c_charge", "c_charge", "c_charge", "c_charge", "c_charge", "c_charge", "c_charge", "c_stumble"], "mode": M_CYCLE, "cycle": 3.75}

## Jeux de clips par style et par état (au plus `MAX_SET` clips ; un clip répété pèse plus
## dans le tirage). AN1b : variantes d'attente, parade, coup par-dessus, impacts, victoire
## (`victory`, camp vainqueur en fin de bataille), cabrage devant les piques (`melee_pikes`),
## trébuchement en charge (`charging` des cavaliers en cycles de six foulées).
const STYLES := {
	"sword": {
		"idle": {"set": ["guard", "idle", "guard", "idle_look", "idle_lean", "idle_helm"]},
		"marching": {"set": ["walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["run"], "speed": 1.05},
		"melee": {"set": ["slash", "thrust", "overhead", "parry", "hit", "hit_b", "hit_c", "guard"], "mode": M_CYCLE, "cycle": 1.3},
		"victory": {"set": ["victory", "victory_b"]},
		"routing": {"set": ["flee", "flee_m"], "speed": 1.1, "fallback": ["run"]},
		"climbing": {"set": ["climb", "guard", "idle"], "mode": M_SPLIT},
	},
	# Lot BV2 : lance, vouge et fourche tenues à deux mains (os `Prop`), comme une pique courte.
	"militia": {
		"idle": {"set": ["pike_idle", "pike_idle", "pike_look"]},
		"victory": {"set": ["victory_pike"]},
		"marching": {"set": ["pike_walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["pike_level_walk"], "speed": 1.3},
		"melee": {"set": ["pike_thrust", "pike_thrust", "pike_level"], "mode": M_CYCLE, "cycle": 1.3},
		"brace": {"set": ["pike_level"]},
		"routing": {"set": ["flee", "flee_m"], "speed": 1.15, "fallback": ["run"]},
		"climbing": {"set": ["climb", "idle", "guard"], "mode": M_SPLIT},
	},
	"pike": {
		"idle": {"set": ["pike_idle", "pike_idle", "pike_look"]},
		"victory": {"set": ["victory_pike"]},
		"marching": {"set": ["pike_walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["pike_level_walk"], "speed": 1.3},
		"melee": {"set": ["pike_thrust", "pike_thrust", "pike_idle"], "mode": M_CYCLE, "cycle": 1.2},
		# Lot BV2 : piques abaissées face à une charge de cavalerie (rendu seulement).
		"brace": {"set": ["pike_level"]},
		"routing": {"set": ["flee", "flee_m"], "speed": 1.1, "fallback": ["run"]},
		"climbing": {"set": ["climb", "pike_idle"], "mode": M_SPLIT},
	},
	"bow": {
		"idle": {"set": ["bow_idle", "idle", "bow_look", "idle_look"]},
		"marching": {"set": ["walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["run"]},
		"shooting": {"set": ["bow_shoot"], "mode": M_VOLLEY, "release": 1.55},
		"melee": {"set": ["slash", "thrust", "parry", "hit", "hit_b", "guard"], "mode": M_CYCLE, "cycle": 1.5},
		"routing": {"set": ["flee", "flee_m"], "speed": 1.15, "fallback": ["run"]},
		"victory": {"set": ["victory", "victory_b"]},
	},
	"crossbow": {
		"idle": {"set": ["xbow_idle", "xbow_idle", "xbow_look"]},
		"marching": {"set": ["walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["run"]},
		"shooting": {"set": ["xbow_shoot"], "mode": M_VOLLEY, "release": 0.3},
		"melee": {"set": ["slash", "thrust", "parry", "hit", "hit_c", "guard"], "mode": M_CYCLE, "cycle": 1.5},
		"routing": {"set": ["flee", "flee_m"], "speed": 1.15, "fallback": ["run"]},
		"victory": {"set": ["victory"]},
	},
	"lance": {
		"idle": {"set": ["c_idle"]},
		"marching": {"set": ["c_walk"]},
		"running": {"set": ["c_gallop"]},
		"charging": CAVALRY_CHARGE,
		"melee": {"set": ["c_thrust", "c_thrust", "c_idle"], "mode": M_CYCLE, "cycle": 1.4},
		"melee_pikes": {"set": ["c_rear", "c_thrust", "c_rear", "c_idle"], "mode": M_CYCLE, "cycle": 1.4},
		"routing": {"set": ["c_gallop"]},
		"victory": {"set": ["c_victory"]},
	},
	"horse_bow": {
		"idle": {"set": ["c_bow_idle"]},
		"marching": {"set": ["c_bow_walk"]},
		"running": {"set": ["c_gallop"]},
		"charging": {"set": ["c_gallop"]},
		"shooting": {"set": ["c_bow_shoot"], "mode": M_VOLLEY, "release": 1.55},
		"melee": {"set": ["c_thrust", "c_bow_idle"], "mode": M_CYCLE, "cycle": 1.4},
		"melee_pikes": {"set": ["c_rear", "c_bow_idle"], "mode": M_CYCLE, "cycle": 1.4},
		"routing": {"set": ["c_gallop"]},
		"victory": {"set": ["c_victory"]},
	},
	## Lot FK2 : civils de la carte vivante (figurines `villager_*`). Les états de travail
	## (`scythe` fauche, `carry` porte un fardeau à l'épaule, `plough` laboure) sont demandés par
	## le rendu de la carte (`state_config(kind, variant, "scythe", false)`) ; sans ces clips (kit
	## grossier antérieur), repli sur la marche ou l'attente.
	"folk": {
		"idle": {"set": ["idle", "idle", "idle_look"]},
		"marching": {"set": ["walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["run"]},
		"routing": {"set": ["flee", "flee_m"], "speed": 1.1, "fallback": ["run"]},
		"victory": {"set": ["victory", "victory_b"]},
		"scythe": {"set": ["scythe"], "fallback": ["idle"]},
		"carry": {"set": ["carry"], "fallback": ["walk"]},
		"plough": {"set": ["plough"], "speed": 0.7, "fallback": ["walk"]},
	},
	## FK2 : porteurs (sac sur l'épaule) : la marche est `carry`.
	"folk_carry": {
		"idle": {"set": ["idle", "idle_look"]},
		"marching": {"set": ["carry"], "fallback": ["walk"]},
		"running": {"set": ["run"]},
		"charging": {"set": ["run"]},
		"routing": {"set": ["flee", "flee_m"], "speed": 1.1, "fallback": ["run"]},
		"carry": {"set": ["carry"], "fallback": ["walk"]},
	},
	## UR2 : jinetes (javelot au lieu de l'arc, cavalerie légère skirmish).
	"horse_javelin": {
		"idle": {"set": ["c_javelin_idle"]},
		"marching": {"set": ["c_javelin_walk"]},
		"running": {"set": ["c_gallop"]},
		"charging": {"set": ["c_gallop"]},
		"shooting": {"set": ["c_javelin_throw"], "mode": M_VOLLEY, "release": 0.69},
		"melee": {"set": ["c_thrust", "c_javelin_idle"], "mode": M_CYCLE, "cycle": 1.4},
		"melee_pikes": {"set": ["c_rear", "c_javelin_idle"], "mode": M_CYCLE, "cycle": 1.4},
		"routing": {"set": ["c_gallop"]},
		"victory": {"set": ["c_victory"]},
	},
}


## Applique `config` (cf. `state_config`) au matériau ; au changement, l'ancienne
## configuration devient la source du fondu.
static func apply_config(mat: ShaderMaterial, config: Dictionary, anim_time: float) -> void:
	var previous: Dictionary = mat.get_meta("v2_config", {})
	if str(previous.get("key", "")) == str(config["key"]):
		return
	if not previous.is_empty():
		mat.set_shader_parameter("prev_set", _ivec(previous["set"]))
		mat.set_shader_parameter("prev_set_size", (previous["set"] as Array).size())
		mat.set_shader_parameter("prev_mode", int(previous["mode"]))
		mat.set_shader_parameter("prev_speed", float(previous["speed"]))
		mat.set_shader_parameter("blend_since", anim_time)
	mat.set_shader_parameter("clip_set", _ivec(config["set"]))
	mat.set_shader_parameter("clip_set_size", (config["set"] as Array).size())
	mat.set_shader_parameter("clip_mode", int(config["mode"]))
	mat.set_shader_parameter("anim_speed", float(config["speed"]))
	mat.set_shader_parameter("cycle_len", float(config["cycle"]))
	mat.set_shader_parameter("release_at", float(config["release"]))
	mat.set_meta("v2_config", config)


## Jeu de clips en `ivec4` : emplacement i dans la composante i % 4, octet i / 4 (AN1b, jusqu'à
## 8 clips ; identique à un indice par composante jusqu'à 4).
static func _ivec(ids: Array) -> Vector4i:
	var v := Vector4i.ZERO
	for i in mini(ids.size(), MAX_SET):
		v[i & 3] = v[i & 3] | ((int(ids[i]) & 255) << ((i >> 2) * 8))
	return v
