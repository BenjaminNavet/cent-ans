class_name BattleImpostors
extends Node

## Imposteurs lointains des figurines skinnées (lot BV3, ADR 0024), rendu seulement.
## Au-delà de `DISTANCE` mètres, un régiment n'est plus dessiné en maillages (LOD2, ≈ 230 à
## 430 triangles) mais en quadrilatères face à la caméra (2 triangles par figurine), texturés
## par un atlas cuit au début de la bataille depuis les figurines V2 elles-mêmes :
## - un atlas par (camp, famille, variante) présent : même matériau que les figurines (livrée,
##   blason), rendu dans un `SubViewport` isolé par une caméra orthographique inclinée ;
## - colonnes : 8 angles de vue autour de la figurine (NT10 : × `COPIES` bandes, variantes de
##   casque et d'habit) ; lignes : 4 jeux (arrêt, marche, course
##   ou charge, action : tir ou mêlée) × 4 images du clip (mode CUSTOM du shader skinné) ;
## - l'image est copiée une fois rendue (mipmaps), le `SubViewport` libéré.
## Le tampon d'instances des figurines (12 flottants, même format) sert tel quel : aucun travail
## GDScript par soldat. `battle_impostor.gdshader` choisit la colonne (angle caméra / cap du
## soldat), l'image (horloge du régiment + phase par soldat) et oriente le quadrilatère.

const SHADER := preload("res://shaders/battle_impostor.gdshader")
## Distance caméra → centre du régiment au-delà de laquelle les imposteurs remplacent le LOD2.
const DISTANCE := 300.0
const COLS := 8
const FRAMES := 4
## Jeux d'images (lignes de l'atlas) : état du régiment → jeu.
const SETS := ["idle", "marching", "charging", "action"]
const PITCH_DEG := 28.0
## Cellule : pixels et mètres (à pied / monté).
const CELL_PX := {false: Vector2i(32, 64), true: Vector2i(64, 64)}
const CELL_M := {false: Vector2(1.3, 2.6), true: Vector2(3.4, 3.4)}
## Marge sous les pieds (fraction de la hauteur de cellule).
const FOOT := 0.06
## NT10 : copies de l'atlas (bandes de 8 colonnes) par figurine : variante (casque...) et habit
## (livrée ou vêtement non teint, teintes) différents ; chaque imposteur en tire une au hasard.
const COPIES := {false: 3, true: 2}
const SHADOW_SHADER := preload("res://shaders/battle_impostor_shadow.gdshader")

## NT10 : `--no-nt10` après `--` : imposteurs de BV3 (une copie, sans ombre ni sang), banc A/B.
static func nt10_enabled() -> bool:
	return not ("--no-nt10" in OS.get_cmdline_user_args())


## NT10 : identifiants de cuisson des `copies` copies : les `round(livery_share × copies)`
## premières portent la livrée, les autres un habit non teint (même test que le shader skinné,
## `h5 < livery_share`, avec une marge contre l'écart du dernier bit du sinus GPU).
static func bake_ids(copies: int, livery_share: float) -> Array:
	var wearing := clampi(roundi(livery_share * float(copies)), 1, copies)
	var ids: Array = []
	var candidate := 1.0
	while ids.size() < copies and candidate < 4000.0:
		var hx := BattleSkinned._hash1(candidate * 1.37 + 0.11)
		var hz := BattleSkinned._hash1(candidate * 0.73 + 1.9)
		var h5 := fposmod(hx * 7.31 + hz * 3.17, 1.0)
		var want_livery := ids.size() < wearing
		if absf(h5 - livery_share) > 0.08 and (h5 < livery_share) == want_livery:
			ids.append(candidate)
		candidate += 1.0
	while ids.size() < copies:
		ids.append(float(ids.size()) + 1.0)
	return ids

var baked_count: int = 0
var _atlases: Dictionary = {}  # key -> {texture, lengths, cell_m, rows}
var _pending: Dictionary = {}  # key -> true (cuisson en cours)


static func key_of(side: String, kind: String, variant: int) -> String:
	return "%s/%s/%d" % [side, kind, variant]


static func state_set(state: String, running: bool) -> int:
	match state:
		"marching":
			return 2 if running else 1
		"charging", "routing":
			return 2
		"melee", "shooting":
			return 3
	return 0


func is_ready(key: String) -> bool:
	return _atlases.has(key)


## Lance la cuisson de l'atlas `key` ; `material` = matériau skinné du régiment (dupliqué).
func request(key: String, kind: String, variant: int, material: ShaderMaterial) -> void:
	if _atlases.has(key) or _pending.has(key):
		return
	_pending[key] = true
	_bake(key, kind, variant, material)


## Matériau d'imposteurs d'un régiment (atlas prêt).
func make_material(key: String) -> ShaderMaterial:
	var atlas: Dictionary = _atlases[key]
	var mat := ShaderMaterial.new()
	mat.shader = SHADER
	mat.set_shader_parameter("atlas", atlas["texture"])
	mat.set_shader_parameter("cell_m", atlas["cell_m"])
	mat.set_shader_parameter("foot", FOOT)
	mat.set_shader_parameter("cols", COLS)
	mat.set_shader_parameter("rows", SETS.size() * FRAMES)
	mat.set_shader_parameter("frames", FRAMES)
	mat.set_shader_parameter("set_len", atlas["lengths"])
	mat.set_shader_parameter("copies", int(atlas.get("copies", 1)))
	return mat


## NT10 : matériau de l'ombre en disque des imposteurs (même MultiMesh, passe multiplicative).
func make_shadow_material(key: String) -> ShaderMaterial:
	var mat := ShaderMaterial.new()
	mat.shader = SHADOW_SHADER
	mat.set_shader_parameter("cell_m", _atlases[key]["cell_m"])
	mat.set_shader_parameter("mounted", (_atlases[key]["cell_m"] as Vector2).x > 2.0)
	return mat


## Quadrilatère 1 × 1 (x −0,5..0,5, y 0..1), déformé par le shader.
static func quad_mesh() -> ArrayMesh:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var corners := [Vector3(-0.5, 0, 0), Vector3(0.5, 0, 0), Vector3(0.5, 1, 0), Vector3(-0.5, 1, 0)]
	for idx in [0, 1, 2, 0, 2, 3]:
		st.set_normal(Vector3(0, 0, 1))
		st.set_uv(Vector2(corners[idx].x + 0.5, 1.0 - corners[idx].y))
		st.add_vertex(corners[idx])
	var mesh := st.commit()
	mesh.custom_aabb = AABB(Vector3(-3, -1, -3), Vector3(6, 6, 6))
	return mesh


func _bake(key: String, kind: String, variant: int, source: ShaderMaterial) -> void:
	var mounted := kind == "cavalry"
	var cell_px: Vector2i = CELL_PX[mounted]
	var cell_m: Vector2 = CELL_M[mounted]
	var rows := SETS.size() * FRAMES
	var copies: int = COPIES[mounted] if nt10_enabled() else 1
	var viewport := SubViewport.new()
	viewport.size = Vector2i(cell_px.x * COLS * copies, cell_px.y * rows)
	viewport.own_world_3d = true
	viewport.transparent_bg = true
	viewport.msaa_3d = Viewport.MSAA_4X
	viewport.render_target_update_mode = SubViewport.UPDATE_ONCE
	add_child(viewport)
	var env := Environment.new()
	env.background_mode = Environment.BG_CLEAR_COLOR
	env.ambient_light_source = Environment.AMBIENT_SOURCE_COLOR
	env.ambient_light_color = Color(1, 1, 1)
	env.ambient_light_energy = 0.7
	env.tonemap_mode = Environment.TONE_MAPPER_LINEAR
	var world_env := WorldEnvironment.new()
	world_env.environment = env
	viewport.add_child(world_env)
	# Un peu de lumière de face et d'en haut : le volume se lit encore à 6 pixels de haut.
	var light := DirectionalLight3D.new()
	light.rotation = Vector3(deg_to_rad(-50), deg_to_rad(20), 0)
	light.light_energy = 0.7
	viewport.add_child(light)
	var pitch := deg_to_rad(PITCH_DEG)
	var forward := Vector3(0, -sin(pitch), -cos(pitch))
	var up := Vector3(0, cos(pitch), -sin(pitch))
	var camera := Camera3D.new()
	camera.projection = Camera3D.PROJECTION_ORTHOGONAL
	camera.keep_aspect = Camera3D.KEEP_HEIGHT
	camera.size = cell_m.y * rows
	camera.near = 1.0
	camera.far = 600.0
	# À 200 m : le shader skinné éclaircit la livrée comme au loin (lisibilité A1-01).
	camera.look_at_from_position(-forward * 200.0, Vector3.ZERO, up)
	camera.current = true
	viewport.add_child(camera)
	# Clips des 4 jeux (premier clip de l'état), durée d'un cycle.
	var ids: Array[int] = []
	var lengths := Vector4.ONE
	var action := "shooting" if kind == "archer" or (kind == "cavalry" and variant == 2) else "melee"
	for s in SETS.size():
		var state: String = action if SETS[s] == "action" else SETS[s]
		var config := BattleSkinned.state_config(kind, variant, state, false)
		ids.append(int((config["set"] as Array)[0]))
		var clip_name := str((config["names"] as Array)[0])
		lengths[s] = maxf(BattleSkinned.clip_seconds(kind, variant, clip_name) / maxf(float(config["speed"]), 0.1), 0.2)
	var mat: ShaderMaterial = source.duplicate()
	mat.set_meta("v2_config", {})
	BattleSkinned.apply_config(mat, {"key": "impostor", "set": ids, "mode": BattleSkinned.M_CUSTOM, "speed": 1.0, "cycle": 1.0, "release": 1.0}, 0.0)
	mat.set_shader_parameter("anim_time", 0.0)
	mat.set_shader_parameter("blend_since", -1000.0)
	mat.set_shader_parameter("interp_distance", 10000.0)
	# FG3 : la caméra est à 200 m, mais l'imposteur cuit le LOD0 avec ses cartes (atlas,
	# tuiles de détail) ; sans effet hors de la variante `FG3_BAKED`.
	mat.set_shader_parameter("fine_distance", 100000.0)
	mat.set_shader_parameter("blood", 0.0)
	mat.set_shader_parameter("hide_pavise", false)
	var width := cell_m.x * COLS * copies
	var height := cell_m.y * rows
	# NT10 : une bande de 8 colonnes par copie, chacune avec son habit et sa variante (matériau
	# dupliqué, `bake_id` du shader skinné) ; une seule copie sans NT10 (atlas de BV3).
	var share: Variant = mat.get_shader_parameter("livery_share")
	var count: Variant = mat.get_shader_parameter("variant_count")
	var ids: Array = bake_ids(copies, 0.7 if share == null else float(share)) if copies > 1 else []
	var variants := 1 if count == null else maxi(int(count), 1)
	for copy in copies:
		var copy_mat := mat
		if copies > 1:
			copy_mat = mat.duplicate()
			copy_mat.set_shader_parameter("bake_id", Vector2(float(ids[copy]), float(copy % variants)))
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.use_custom_data = true
		mm.mesh = BattleSkinned.mesh(kind, variant, 0)
		mm.instance_count = COLS * rows
		for r in rows:
			var s := r / FRAMES
			var f := r % FRAMES
			for c in COLS:
				var x := (float(copy * COLS + c) + 0.5) * cell_m.x - width * 0.5
				var y := height * 0.5 - (float(r) + 1.0) * cell_m.y + FOOT * cell_m.y
				var basis := Basis(Vector3.UP, float(c) * TAU / float(COLS))
				var k := r * COLS + c
				mm.set_instance_transform(k, Transform3D(basis, Vector3(x, 0, 0) + up * y))
				mm.set_instance_custom_data(k, Color(-float(f) / float(FRAMES) * lengths[s], float(s), 0.0, 0.0))
		var figures := MultiMeshInstance3D.new()
		figures.multimesh = mm
		figures.material_override = copy_mat
		viewport.add_child(figures)
	await RenderingServer.frame_post_draw
	if _bake_aborted(key, viewport):
		return
	await get_tree().process_frame
	if _bake_aborted(key, viewport):
		return
	await RenderingServer.frame_post_draw
	if _bake_aborted(key, viewport):
		return
	var image := viewport.get_texture().get_image()
	if image == null or image.is_empty():
		# Sans rendu (headless) : pas d'atlas, le LOD2 reste dessiné.
		_pending.erase(key)
		viewport.queue_free()
		return
	image.generate_mipmaps()
	_atlases[key] = {"texture": ImageTexture.create_from_image(image), "lengths": lengths, "cell_m": cell_m, "image": image, "copies": copies}
	_pending.erase(key)
	baked_count += 1
	viewport.queue_free()


## Cuisson interrompue (nœud sorti de l'arbre ou libéré pendant un `await`) : on abandonne
## proprement, l'atlas pourra être redemandé.
func _bake_aborted(key: String, viewport: SubViewport) -> bool:
	if is_instance_valid(self) and is_inside_tree():
		return false
	_pending.erase(key)
	if is_instance_valid(viewport):
		viewport.queue_free()
	return true


## Image de l'atlas (tests, captures).
func atlas_image(key: String) -> Image:
	return _atlases.get(key, {}).get("image")
