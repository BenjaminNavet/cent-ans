class_name RenderQuality
extends RefCounted

## Préréglages de qualité du rendu (lot V3, A1-14) : Basse, Moyenne, Haute (par défaut), Ultra.
## Réglage `video/quality` de l'autoload `Settings` (Réglages > Affichage), ou `--quality=<niveau>`
## en ligne de commande (mesures, captures). Règle les coûts globaux (atlas d'ombres, filtre doux,
## qualité SSAO/SSIL, grille du brouillard volumétrique, MSAA) et, par environnement enregistré,
## les effets : SSAO, lumière rebondie en espace écran (SSIL), SDFGI (Ultra, bataille), brouillard
## volumétrique (lumière qui « traverse » la brume), halo, portée et cascades des ombres.
## Lot PF1 : chaque niveau règle aussi la géométrie et les effets de scène (clés « scène » des
## préréglages) : relief fin de la carte (présence, biais de LOD des blocs), portée et densité de
## la végétation et de ses ombres, portée et cascades des ombres de la carte, filtre des ombres
## douces de la carte, distances de LOD et d'imposteurs des figurines de bataille, rayon de
## l'herbe de bataille, nombre de particules (météo, sang, fumée, poussière).
## Les nœuds concernés s'inscrivent dans le groupe `CLIENT_GROUP` et reçoivent
## `apply_render_quality(preset)` à chaque changement ; les particules sont réduites à leur
## création (`SceneTree.node_added`). Purement visuel.

const LEVELS: Array[String] = ["low", "medium", "high", "ultra"]
const LABELS: Array[String] = ["Basse", "Moyenne", "Haute", "Ultra"]
const DEFAULT_LEVEL := "high"
## RL1 : valeur du réglage tant que le joueur n'a rien choisi : niveau déduit du GPU détecté.
const AUTO := "auto"
const GROUP := "render_quality"
## PF1 : nœuds qui implémentent `apply_render_quality(preset: Dictionary)`.
const CLIENT_GROUP := "render_quality_client"
## Clés « scène » (PF1), communes à tous les niveaux :
## - quadtree de relief (ZG2, pyramide en cache) : `relief_vertex_px` espacement maximal des
##   sommets à l'écran, `relief_items` budget de nœuds, `relief_extra_depth` profondeur au-delà
##   de l'étage de données, `relief_pages` couches de pages (VRAM, au chargement de la carte),
##   `relief_shadow_cascades` cascades du soleil où le relief porte une ombre (au-delà, il en
##   reçoit seulement) ;
## - `fine_relief` : sans pyramide, relief fin `FineTerrainJob` au zoom comté (repli) ;
## - `terrain_near` : facteur sur la distance du LOD proche du relief de la carte ;
## - `veg_density` : part des arbres affichés ; `veg_detail` : facteur sur la portée des arbres
##   détaillés ; `veg_shadow_distance` : zoom au-delà duquel les arbres ne portent plus d'ombre
##   (0 : jamais d'ombre) ;
## - `map_shadow_range`, `map_shadow_splits`, `map_soft_shadows` : ombres de la carte ;
## - `battle_lod` : facteur sur les distances de LOD, d'ombre et d'imposteurs des soldats ;
## - `grass` : facteur sur le rayon de l'herbe de bataille ; `particles` : part des particules.
const SCENE_KEYS := ["relief_vertex_px", "relief_items", "relief_extra_depth", "relief_pages", "relief_shadow_cascades", "fine_relief", "terrain_near", "veg_density", "veg_detail",
	"veg_shadow_distance", "map_shadow_range", "map_shadow_splits", "map_soft_shadows", "battle_lod",
	"grass", "particles"]

## Coûts par niveau. `shadow_distance` : facteur sur la portée d'ombre demandée par la scène.
## `volumetric` : "off", "weather" (brouillard, pluie, neige seulement) ou "always".
const PRESETS := {
	# Réglages d'avant le lot V3 (project.godot) : référence des mesures (`--bench-ab=`), hors menu.
	"legacy": {
		"msaa": Viewport.MSAA_2X, "shadow_atlas": 8192, "soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_ULTRA,
		"shadow_splits": 4, "shadow_distance": 1.0, "ssao": true, "ssao_quality": RenderingServer.ENV_SSAO_QUALITY_ULTRA,
		"ssil": false, "ssil_quality": RenderingServer.ENV_SSIL_QUALITY_LOW, "volumetric": "off", "sdfgi": false,
		"glow": true, "fog_grid": [64, 32],
		"relief_vertex_px": 4.0, "relief_items": 700, "relief_extra_depth": 3, "relief_pages": 256, "relief_shadow_cascades": 4,
		"fine_relief": true, "terrain_near": 1.0, "veg_density": 1.0, "veg_detail": 1.0,
		"veg_shadow_distance": 300.0, "map_shadow_range": 1.0, "map_shadow_splits": 4,
		"map_soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_ULTRA, "battle_lod": 1.0, "grass": 1.0, "particles": 1.0,
	},
	"low": {
		"msaa": Viewport.MSAA_DISABLED, "shadow_atlas": 2048, "soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_LOW,
		"shadow_splits": 2, "shadow_distance": 0.6, "ssao": false, "ssao_quality": RenderingServer.ENV_SSAO_QUALITY_LOW,
		"ssil": false, "ssil_quality": RenderingServer.ENV_SSIL_QUALITY_LOW, "volumetric": "off", "sdfgi": false,
		"glow": false, "fog_grid": [64, 32],
		"relief_vertex_px": 12.0, "relief_items": 350, "relief_extra_depth": 2, "relief_pages": 128, "relief_shadow_cascades": 1,
		"fine_relief": false, "terrain_near": 0.6, "veg_density": 0.5, "veg_detail": 0.6,
		"veg_shadow_distance": 0.0, "map_shadow_range": 0.6, "map_shadow_splits": 2,
		"map_soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_LOW, "battle_lod": 0.55, "grass": 0.55, "particles": 0.35,
	},
	"medium": {
		"msaa": Viewport.MSAA_2X, "shadow_atlas": 4096, "soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_MEDIUM,
		"shadow_splits": 4, "shadow_distance": 0.8, "ssao": true, "ssao_quality": RenderingServer.ENV_SSAO_QUALITY_MEDIUM,
		"ssil": false, "ssil_quality": RenderingServer.ENV_SSIL_QUALITY_LOW, "volumetric": "off", "sdfgi": false,
		"glow": true, "fog_grid": [64, 32],
		"relief_vertex_px": 8.0, "relief_items": 500, "relief_extra_depth": 3, "relief_pages": 192, "relief_shadow_cascades": 1,
		"fine_relief": true, "terrain_near": 0.8, "veg_density": 0.75, "veg_detail": 0.8,
		"veg_shadow_distance": 200.0, "map_shadow_range": 0.8, "map_shadow_splits": 4,
		"map_soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_LOW, "battle_lod": 0.75, "grass": 0.75, "particles": 0.6,
	},
	"high": {
		"msaa": Viewport.MSAA_2X, "shadow_atlas": 8192, "soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_HIGH,
		"shadow_splits": 4, "shadow_distance": 1.0, "ssao": true, "ssao_quality": RenderingServer.ENV_SSAO_QUALITY_HIGH,
		"ssil": true, "ssil_quality": RenderingServer.ENV_SSIL_QUALITY_LOW, "volumetric": "weather", "sdfgi": false,
		"glow": true, "fog_grid": [64, 48],
		# PF1 : filtre moyen sur la carte (ombres douces à contact durci, PCSS) : invisible au
		# zoom comté, ≈ 4 ms de GPU de moins qu'en « haut ». Quadtree de relief à 6 px par sommet (ZG2 : 4,
		# gardé en Ultra) : aucune différence visible au zoom comté, ≈ 6 ms de GPU de moins.
		"relief_vertex_px": 6.0, "relief_items": 700, "relief_extra_depth": 3, "relief_pages": 256, "relief_shadow_cascades": 1,
		"fine_relief": true, "terrain_near": 1.0, "veg_density": 1.0, "veg_detail": 1.0,
		"veg_shadow_distance": 300.0, "map_shadow_range": 1.0, "map_shadow_splits": 4,
		"map_soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_MEDIUM, "battle_lod": 1.0, "grass": 1.0, "particles": 1.0,
	},
	"ultra": {
		"msaa": Viewport.MSAA_4X, "shadow_atlas": 8192, "soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_ULTRA,
		"shadow_splits": 4, "shadow_distance": 1.35, "ssao": true, "ssao_quality": RenderingServer.ENV_SSAO_QUALITY_ULTRA,
		"ssil": true, "ssil_quality": RenderingServer.ENV_SSIL_QUALITY_HIGH, "volumetric": "always", "sdfgi": true,
		"glow": true, "fog_grid": [128, 64],
		"relief_vertex_px": 4.0, "relief_items": 900, "relief_extra_depth": 3, "relief_pages": 256, "relief_shadow_cascades": 2,
		"fine_relief": true, "terrain_near": 1.2, "veg_density": 1.0, "veg_detail": 1.3,
		"veg_shadow_distance": 400.0, "map_shadow_range": 1.2, "map_shadow_splits": 4,
		"map_soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_HIGH, "battle_lod": 1.3, "grass": 1.2, "particles": 1.0,
	},
}


## Niveau imposé par le banc A/B (`BattleScene`, `--bench-ab=`), "" sinon.
static var override_level: String = ""
## PF1 : valeurs courantes lues à chaque image par le rendu des batailles (évite un
## dictionnaire par régiment et par image).
static var battle_lod_scale: float = 1.0
static var particle_ratio: float = 1.0
## Contexte du dernier environnement enregistré encore présent ("battle" ou "campaign") : le
## filtre des ombres douces est global au serveur de rendu.
static var active_context: String = "battle"
static var _particle_hook: bool = false


## Niveau courant : `--quality=` puis réglage `video/quality`, sinon `DEFAULT_LEVEL`.
static func current() -> String:
	if override_level != "":
		return override_level
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--quality="):
			var forced := arg.trim_prefix("--quality=")
			if PRESETS.has(forced):
				return forced
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var settings := tree.root.get_node_or_null("Settings")
		if settings != null:
			var level := str(settings.call("get_value", "video/quality"))
			if level in LEVELS:
				return level
			if level == AUTO:
				return detected_level()
	return DEFAULT_LEVEL


## RL1 : niveau conseillé pour le GPU de cette machine (réglage « Automatique », par défaut).
static func detected_level() -> String:
	return level_for_adapter(RenderingServer.get_video_adapter_name())


## Haute sur Apple Silicon récent (M2 et suivants, ou M1 Pro/Max/Ultra), Moyenne sinon
## (M1 de base, GPU intégrés Intel, adaptateur inconnu). Purement indicatif : le joueur choisit.
static func level_for_adapter(adapter: String) -> String:
	var name := adapter.to_lower().strip_edges()
	if not name.begins_with("apple m"):
		return "medium"
	var digits := ""
	for character in name.substr(7):
		if character >= "0" and character <= "9":
			digits += character
		else:
			break
	if digits == "":
		return "medium"
	var generation := int(digits)
	if generation >= 2:
		return "high"
	for tier in ["pro", "max", "ultra"]:
		if name.contains(tier):
			return "high"
	return "medium"


static func preset(level: String = "") -> Dictionary:
	return PRESETS.get(level if level != "" else current(), PRESETS[DEFAULT_LEVEL])


## Réglages globaux du serveur de rendu et du viewport principal.
static func apply_global(viewport: Viewport = null) -> void:
	var p := preset()
	battle_lod_scale = float(p["battle_lod"])
	particle_ratio = float(p["particles"])
	RenderingServer.directional_shadow_atlas_set_size(int(p["shadow_atlas"]), true)
	var soft: int = p["map_soft_shadows"] if active_context == "campaign" else p["soft_shadows"]
	RenderingServer.directional_soft_shadow_filter_set_quality(soft)
	RenderingServer.positional_soft_shadow_filter_set_quality(p["soft_shadows"])
	RenderingServer.environment_set_ssao_quality(p["ssao_quality"], true, 0.5, 2, 50.0, 300.0)
	# SSIL en demi-résolution : la lumière rebondie est basse fréquence, le flou la lisse.
	RenderingServer.environment_set_ssil_quality(p["ssil_quality"], true, 0.5, 4, 50.0, 300.0)
	var grid: Array = p["fog_grid"]
	RenderingServer.environment_set_volumetric_fog_volume_size(int(grid[0]), int(grid[1]))
	RenderingServer.environment_set_volumetric_fog_filter_active(true)
	if viewport == null:
		var tree := Engine.get_main_loop() as SceneTree
		viewport = tree.root if tree != null else null
	if viewport != null:
		viewport.msaa_3d = p["msaa"]
		_install_particle_hook(viewport.get_tree())


## PF1 : particules réduites selon le niveau dès leur entrée dans l'arbre (`amount` d'origine
## gardé en méta : un changement de niveau le réapplique sans cumul).
static func _install_particle_hook(tree: SceneTree) -> void:
	if _particle_hook or tree == null:
		return
	_particle_hook = true
	tree.node_added.connect(_on_node_added)


## Différé : les scripts règlent souvent `amount` juste après `add_child`. Les particules d'un
## effet ponctuel (ex. incendie) peuvent être libérées avant que l'appel différé ne s'exécute ;
## passer l'objet directement à `call_deferred` fait alors échouer la file de messages (« Cannot
## convert argument 1 from Object to Object », le slot mémoire étant réutilisé par un autre objet
## d'ici là). On passe donc l'ID d'instance et on ne résout le nœud qu'au moment de l'appel.
static func _on_node_added(node: Node) -> void:
	if node is GPUParticles3D or node is CPUParticles3D:
		_scale_particles_by_id.call_deferred(node.get_instance_id())


static func _scale_particles_by_id(id: int) -> void:
	var node := instance_from_id(id)
	if node != null:
		scale_particles(node)


static func scale_particles(node: Node) -> void:
	if not is_instance_valid(node) or not (node is GPUParticles3D or node is CPUParticles3D):
		return
	if not node.has_meta("rq_amount"):
		node.set_meta("rq_amount", int(node.get("amount")))
	var wanted := maxi(1, int(round(int(node.get_meta("rq_amount")) * particle_ratio)))
	if int(node.get("amount")) != wanted:
		node.set("amount", wanted)


## Enregistre un environnement (et son soleil) : il sera réappliqué quand le réglage change.
## `context` : "battle" ou "campaign" ; `weather` : clé météo de la bataille ("" en campagne).
static func register(world_env: WorldEnvironment, sun: DirectionalLight3D, context: String, weather: String = "") -> void:
	active_context = context
	if not world_env.tree_exiting.is_connected(_on_env_exiting):
		world_env.tree_exiting.connect(_on_env_exiting.bind(world_env))
	world_env.set_meta("render_context", context)
	world_env.set_meta("render_weather", weather)
	world_env.set_meta("render_sun", sun)
	if sun != null and not sun.has_meta("base_shadow_distance"):
		sun.set_meta("base_shadow_distance", sun.directional_shadow_max_distance)
	if not world_env.is_in_group(GROUP):
		world_env.add_to_group(GROUP)
	apply_global(world_env.get_viewport())
	apply_environment(world_env)


## Effets de l'environnement enregistré selon le niveau courant.
static func apply_environment(world_env: WorldEnvironment) -> void:
	var env := world_env.environment
	if env == null:
		return
	var p := preset()
	var context := str(world_env.get_meta("render_context", "battle"))
	var weather := str(world_env.get_meta("render_weather", ""))
	env.ssao_enabled = bool(p["ssao"])
	env.ssil_enabled = bool(p["ssil"])
	if env.ssil_enabled:
		env.ssil_radius = 6.0 if context == "battle" else 12.0
		env.ssil_intensity = 0.9
		env.ssil_sharpness = 0.98
		env.ssil_normal_rejection = 1.0
	env.glow_enabled = bool(p["glow"])
	env.sdfgi_enabled = bool(p["sdfgi"]) and context == "battle"
	if env.sdfgi_enabled:
		env.sdfgi_use_occlusion = true
		env.sdfgi_cascades = 4
		env.sdfgi_min_cell_size = 0.4
		env.sdfgi_energy = 0.8
		env.sdfgi_bounce_feedback = 0.4
	var volumetric := str(p["volumetric"])
	var foggy := weather in ["fog", "rain", "snow"]
	env.volumetric_fog_enabled = context == "battle" and (volumetric == "always" or (volumetric == "weather" and foggy))
	var sun := world_env.get_meta("render_sun", null) as DirectionalLight3D
	# En campagne, la portée des ombres suit le zoom (`CampaignAtmosphere`).
	if sun != null and is_instance_valid(sun) and context == "battle":
		apply_sun(sun, p)


static func apply_sun(sun: DirectionalLight3D, p: Dictionary) -> void:
	var base := float(sun.get_meta("base_shadow_distance", sun.directional_shadow_max_distance))
	sun.directional_shadow_max_distance = base * float(p["shadow_distance"])
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS if int(p["shadow_splits"]) >= 4 else DirectionalLight3D.SHADOW_PARALLEL_2_SPLITS


## Un environnement quitte l'arbre (fin de bataille) : le contexte redevient celui d'un
## environnement restant (la carte, sous la bataille).
static func _on_env_exiting(world_env: WorldEnvironment) -> void:
	var tree := world_env.get_tree()
	if tree == null:
		return
	var context := ""
	for node in tree.get_nodes_in_group(GROUP):
		if node != world_env and node.is_inside_tree():
			context = str(node.get_meta("render_context", context))
	if context != "" and context != active_context:
		active_context = context
		apply_global(tree.root)


## Réapplique le niveau courant à tous les environnements enregistrés (réglage modifié) et aux
## nœuds inscrits dans `CLIENT_GROUP` (PF1).
static func reapply(tree: SceneTree) -> void:
	apply_global(tree.root)
	for node in tree.get_nodes_in_group(GROUP):
		if node is WorldEnvironment:
			apply_environment(node)
	apply_clients(tree)


static func apply_clients(tree: SceneTree) -> void:
	var p := preset()
	for node in tree.get_nodes_in_group(CLIENT_GROUP):
		if node.has_method("apply_render_quality"):
			node.call("apply_render_quality", p)
	for node in tree.root.find_children("*", "GPUParticles3D", true, false):
		scale_particles(node)
	for node in tree.root.find_children("*", "CPUParticles3D", true, false):
		scale_particles(node)
