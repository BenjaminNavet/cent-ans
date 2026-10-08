class_name RenderQuality
extends RefCounted

## Préréglages de qualité du rendu : Basse, Moyenne, Haute (par défaut), Ultra, lus dans
## `data/fx/render_quality.json` (clés documentées par `data/schemas/render_quality.schema.json`).
## Niveau = réglage `video/quality` de `Settings`, ou `--quality=<niveau>` (mesures, captures).
## Un préréglage règle les coûts globaux (ombres, SSAO/SSIL, brouillard volumétrique, MSAA,
## mise à l'échelle 3D) et la géométrie de la scène (relief, végétation, LOD, herbe, particules).
## Les nœuds du groupe `CLIENT_GROUP` reçoivent `apply_render_quality(preset)` à chaque
## changement ; les particules sont réduites à leur création. Purement visuel.

const LEVELS: Array[String] = ["low", "medium", "high", "ultra"]
const LABELS: Array[String] = ["Basse", "Moyenne", "Haute", "Ultra"]
const DEFAULT_LEVEL := "high"
## Valeur du réglage tant que le joueur n'a rien choisi : niveau déduit du GPU détecté.
const AUTO := "auto"
const GROUP := "render_quality"
## Nœuds qui implémentent `apply_render_quality(preset: Dictionary)`.
const CLIENT_GROUP := "render_quality_client"
const DATA_PATH := "fx/render_quality.json"

## Modes de mise à l'échelle 3D du viewport racine : moteur sous Metal, moteur ailleurs (FSR 1 au
## lieu du MetalFX spatial, FSR 2 au lieu du temporel), nom court des bancs (`--upscale=`) et
## anticrénelage intégré (le temporel remplace MSAA et FXAA). `bilinear` : référence des bancs.
const UPSCALE_OFF := "off"
const UPSCALE_SPATIAL := "metalfx_spatial"
const UPSCALE_TEMPORAL := "metalfx_temporal"
const UPSCALE_MODES := {
	"off": {"metal": Viewport.SCALING_3D_MODE_BILINEAR, "other": Viewport.SCALING_3D_MODE_BILINEAR, "short": "off"},
	"metalfx_spatial": {"metal": Viewport.SCALING_3D_MODE_METALFX_SPATIAL, "other": Viewport.SCALING_3D_MODE_FSR, "short": "metalfx_s"},
	"metalfx_temporal": {"metal": Viewport.SCALING_3D_MODE_METALFX_TEMPORAL, "other": Viewport.SCALING_3D_MODE_FSR2, "short": "metalfx_t"},
	"bilinear": {"metal": Viewport.SCALING_3D_MODE_BILINEAR, "other": Viewport.SCALING_3D_MODE_BILINEAR, "short": "bilinear"},
}
## Réglage joueur `video/upscale` : "auto" suit le préréglage de qualité.
const UPSCALE_CHOICES: Array[String] = ["auto", "off", "quality", "performance"]
const UPSCALE_LABELS: Array[String] = ["Automatique", "Désactivée", "MetalFX qualité", "MetalFX performance"]

## Valeurs du fichier de données (chaînes) -> énumérations du moteur.
const MSAA_BY_NAME := {"off": Viewport.MSAA_DISABLED, "2x": Viewport.MSAA_2X, "4x": Viewport.MSAA_4X}
const SHADOW_QUALITY_BY_NAME := {
	"low": RenderingServer.SHADOW_QUALITY_SOFT_LOW, "medium": RenderingServer.SHADOW_QUALITY_SOFT_MEDIUM,
	"high": RenderingServer.SHADOW_QUALITY_SOFT_HIGH, "ultra": RenderingServer.SHADOW_QUALITY_SOFT_ULTRA,
}
const SSAO_QUALITY_BY_NAME := {
	"low": RenderingServer.ENV_SSAO_QUALITY_LOW, "medium": RenderingServer.ENV_SSAO_QUALITY_MEDIUM,
	"high": RenderingServer.ENV_SSAO_QUALITY_HIGH, "ultra": RenderingServer.ENV_SSAO_QUALITY_ULTRA,
}
const SSIL_QUALITY_BY_NAME := {"low": RenderingServer.ENV_SSIL_QUALITY_LOW, "high": RenderingServer.ENV_SSIL_QUALITY_HIGH}
const INT_KEYS := ["shadow_atlas", "shadow_splits", "relief_items", "relief_extra_depth", "relief_pages", "relief_shadow_cascades", "map_shadow_splits"]

## Niveau imposé par le banc A/B (`BattleScene`, `--bench-ab=`), "" sinon.
static var override_level: String = ""
## Valeurs courantes lues à chaque image par le rendu des batailles (évite un
## dictionnaire par régiment et par image).
static var battle_lod_scale: float = 1.0
static var particle_ratio: float = 1.0
## Contexte du dernier environnement enregistré encore présent ("battle" ou "campaign") : le
## filtre des ombres douces est global au serveur de rendu.
static var active_context: String = "battle"
static var _particle_hook: bool = false
## Mise à l'échelle imposée par un banc (`metalfx_s:0.75`, `metalfx_t:0.67`,
## `bilinear:0.75`, `off`), "" sinon.
static var upscale_override: String = ""
static var _presets: Dictionary = {}


## Niveau courant : `--quality=` puis réglage `video/quality`, sinon `DEFAULT_LEVEL`.
static func current() -> String:
	if override_level != "":
		return override_level
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--quality="):
			var forced := arg.trim_prefix("--quality=")
			if presets().has(forced):
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


## Niveau conseillé pour le GPU de cette machine (réglage « Automatique », par défaut).
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


## Tous les préréglages (`low`, `medium`, `high`, `ultra`, `legacy`), énumérations du moteur résolues.
static func presets() -> Dictionary:
	if _presets.is_empty():
		for level: String in data()["presets"]:
			_presets[level] = _resolve(data()["presets"][level])
	return _presets


static func data() -> Dictionary:
	return DataFile.load_cached(DATA_PATH)


static func _resolve(raw: Dictionary) -> Dictionary:
	var p: Dictionary = raw.duplicate()
	p.erase("note")
	for key: String in INT_KEYS:
		p[key] = int(p[key])
	p["msaa"] = MSAA_BY_NAME[p["msaa"]]
	p["map_msaa"] = MSAA_BY_NAME[p["map_msaa"]]
	p["soft_shadows"] = SHADOW_QUALITY_BY_NAME[p["soft_shadows"]]
	p["map_soft_shadows"] = SHADOW_QUALITY_BY_NAME[p["map_soft_shadows"]]
	p["ssao_quality"] = SSAO_QUALITY_BY_NAME[p["ssao_quality"]]
	p["ssil_quality"] = SSIL_QUALITY_BY_NAME[p["ssil_quality"]]
	return p


static func preset(level: String = "") -> Dictionary:
	return presets().get(level if level != "" else current(), presets()[DEFAULT_LEVEL])


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
		apply_upscale(viewport, p)
		_install_particle_hook(viewport.get_tree())


## Mode et échelle de la mise à l'échelle 3D : banc, puis réglage du joueur, sinon préréglage.
## `pixels` : pixels physiques du viewport 3D (0 : taille de la fenêtre principale).
static func upscale(p: Dictionary = {}, pixels: float = 0.0) -> Dictionary:
	if upscale_override != "":
		return parse_upscale(upscale_override)
	for arg in OS.get_cmdline_user_args():
		if arg.begins_with("--upscale="):  # captures et mesures : `--upscale=metalfx_t:0.75`
			return parse_upscale(arg.trim_prefix("--upscale="))
	if p.is_empty():
		p = preset()
	var choice := "auto"
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		var settings := tree.root.get_node_or_null("Settings")
		if settings != null:
			choice = str(settings.call("get_value", "video/upscale"))
	var player_choices: Dictionary = data()["upscale"]["player_choices"]
	if player_choices.has(choice):
		return {"mode": str(player_choices[choice]["mode"]), "scale": float(player_choices[choice]["scale"])}
	var up := preset_upscale(p)
	if pixels <= 0.0:
		var window_size := DisplayServer.window_get_size()
		pixels = float(window_size.x * window_size.y)
	up["scale"] = budget_scale(str(up["mode"]), float(up["scale"]), pixels)
	return up


## Les échelles des préréglages sont réglées pour une définition de référence (1920 x 1080). Au-delà
## (Retina, 4K), « Automatique » garde le même nombre de pixels rendus, sans descendre sous
## `min_upscale_scale`. Les choix explicites du joueur restent tels quels.
static func budget_scale(mode: String, scale: float, pixels: float) -> float:
	var reference := float(data()["upscale"]["reference_pixels"])
	if mode == UPSCALE_OFF or pixels <= reference:
		return scale
	return maxf(min_upscale_scale(), minf(scale, scale * sqrt(reference / pixels)))


static func min_upscale_scale() -> float:
	return float(data()["upscale"]["min_scale"])


## Mise à l'échelle prévue par un préréglage (choix « Automatique »).
static func preset_upscale(p: Dictionary) -> Dictionary:
	var mode := str(p.get("upscale_mode", UPSCALE_OFF))
	return {"mode": mode, "scale": float(p.get("upscale_scale", 1.0)) if mode != UPSCALE_OFF else 1.0}


## Libellé court d'une mise à l'échelle (menu Réglages).
static func upscale_label(up: Dictionary) -> String:
	var mode := str(up["mode"])
	if mode == UPSCALE_OFF or float(up["scale"]) >= 1.0 and mode != UPSCALE_TEMPORAL:
		return "désactivée"
	var name := "MetalFX" if is_metal() else "FSR"
	return "%s %s %d %%" % [name, "temporel" if mode == UPSCALE_TEMPORAL else "spatial", roundi(float(up["scale"]) * 100.0)]


## `metalfx_s:<échelle>`, `metalfx_t:<échelle>`, `bilinear:<échelle>` ou `off` (bancs A/B).
static func parse_upscale(config: String) -> Dictionary:
	var parts := config.split(":")
	var mode := UPSCALE_OFF
	for mode_name: String in UPSCALE_MODES:
		if UPSCALE_MODES[mode_name]["short"] == parts[0]:
			mode = mode_name
	var scale := float(parts[1]) if parts.size() > 1 else 1.0
	return {"mode": mode, "scale": clampf(scale, 0.25, 1.0) if mode != UPSCALE_OFF else 1.0}


## Mode de mise à l'échelle du moteur : MetalFX sous Metal, repli FSR 1 / FSR 2 ailleurs.
static func scaling_mode_for(mode: String, metal: bool) -> Viewport.Scaling3DMode:
	return UPSCALE_MODES[mode]["metal" if metal else "other"] as Viewport.Scaling3DMode


static func is_metal() -> bool:
	return RenderingServer.get_current_rendering_driver_name() == "metal"


## Vrai si le mode fait aussi l'anticrénelage (MetalFX temporel, FSR 2).
static func is_temporal(scaling_mode: int) -> bool:
	return scaling_mode == Viewport.SCALING_3D_MODE_METALFX_TEMPORAL or scaling_mode == Viewport.SCALING_3D_MODE_FSR2


static func apply_upscale(viewport: Viewport, p: Dictionary) -> void:
	var size := Vector2((viewport as Window).size) if viewport is Window else viewport.get_visible_rect().size
	var up := upscale(p, size.x * size.y)
	var scaling_mode := scaling_mode_for(str(up["mode"]), is_metal())
	viewport.scaling_3d_mode = scaling_mode
	viewport.scaling_3d_scale = float(up["scale"])
	# Le temporel accumule les images précédentes : il remplace MSAA et FXAA (sinon coût double
	# et flou). Hors temporel, FXAA suit project.godot.
	var temporal := is_temporal(scaling_mode)
	viewport.msaa_3d = Viewport.MSAA_DISABLED if temporal else msaa_for(p, active_context)
	var fxaa := int(ProjectSettings.get_setting("rendering/anti_aliasing/quality/screen_space_aa", 0))
	viewport.screen_space_aa = Viewport.SCREEN_SPACE_AA_DISABLED if temporal else fxaa as Viewport.ScreenSpaceAA


## MSAA du contexte : `map_msaa` sur la carte de campagne (shader du terrain trop
## lourd pour être ombré deux fois aux bords des triangles : −11 ms à 50 % d'échelle, FXAA garde
## les bords lisses), `msaa` en bataille.
static func msaa_for(p: Dictionary, context: String) -> Viewport.MSAA:
	return (p.get("map_msaa", p["msaa"]) if context == "campaign" else p["msaa"]) as Viewport.MSAA


## Particules réduites selon le niveau dès leur entrée dans l'arbre (`amount` d'origine
## gardé en méta : un changement de niveau le réapplique sans cumul).
static func _install_particle_hook(tree: SceneTree) -> void:
	if _particle_hook or tree == null:
		return
	_particle_hook = true
	tree.node_added.connect(_on_node_added)
	# Le budget de pixels dépend de la taille de la fenêtre (plein écran, autre écran).
	tree.root.size_changed.connect(_on_root_resized)


static func _on_root_resized() -> void:
	var tree := Engine.get_main_loop() as SceneTree
	if tree != null and tree.root != null:
		apply_upscale(tree.root, preset())


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
## nœuds inscrits dans `CLIENT_GROUP`.
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
