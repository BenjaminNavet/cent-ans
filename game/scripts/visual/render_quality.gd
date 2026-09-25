class_name RenderQuality
extends RefCounted

## Préréglages de qualité du rendu (lot V3, A1-14) : Basse, Moyenne, Haute (par défaut), Ultra.
## Réglage `video/quality` de l'autoload `Settings` (Réglages > Affichage), ou `--quality=<niveau>`
## en ligne de commande (mesures, captures). Règle les coûts globaux (atlas d'ombres, filtre doux,
## qualité SSAO/SSIL, grille du brouillard volumétrique, MSAA) et, par environnement enregistré,
## les effets : SSAO, lumière rebondie en espace écran (SSIL), SDFGI (Ultra, bataille), brouillard
## volumétrique (lumière qui « traverse » la brume), halo, portée et cascades des ombres.
## Purement visuel.

const LEVELS: Array[String] = ["low", "medium", "high", "ultra"]
const LABELS: Array[String] = ["Basse", "Moyenne", "Haute", "Ultra"]
const DEFAULT_LEVEL := "high"
## RL1 : valeur du réglage tant que le joueur n'a rien choisi : niveau déduit du GPU détecté.
const AUTO := "auto"
const GROUP := "render_quality"

## Coûts par niveau. `shadow_distance` : facteur sur la portée d'ombre demandée par la scène.
## `volumetric` : "off", "weather" (brouillard, pluie, neige seulement) ou "always".
const PRESETS := {
	# Réglages d'avant le lot V3 (project.godot) : référence des mesures (`--bench-ab=`), hors menu.
	"legacy": {
		"msaa": Viewport.MSAA_2X, "shadow_atlas": 8192, "soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_ULTRA,
		"shadow_splits": 4, "shadow_distance": 1.0, "ssao": true, "ssao_quality": RenderingServer.ENV_SSAO_QUALITY_ULTRA,
		"ssil": false, "ssil_quality": RenderingServer.ENV_SSIL_QUALITY_LOW, "volumetric": "off", "sdfgi": false,
		"glow": true, "fog_grid": [64, 32],
	},
	"low": {
		"msaa": Viewport.MSAA_DISABLED, "shadow_atlas": 2048, "soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_LOW,
		"shadow_splits": 2, "shadow_distance": 0.6, "ssao": false, "ssao_quality": RenderingServer.ENV_SSAO_QUALITY_LOW,
		"ssil": false, "ssil_quality": RenderingServer.ENV_SSIL_QUALITY_LOW, "volumetric": "off", "sdfgi": false,
		"glow": false, "fog_grid": [64, 32],
	},
	"medium": {
		"msaa": Viewport.MSAA_2X, "shadow_atlas": 4096, "soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_MEDIUM,
		"shadow_splits": 4, "shadow_distance": 0.8, "ssao": true, "ssao_quality": RenderingServer.ENV_SSAO_QUALITY_MEDIUM,
		"ssil": false, "ssil_quality": RenderingServer.ENV_SSIL_QUALITY_LOW, "volumetric": "off", "sdfgi": false,
		"glow": true, "fog_grid": [64, 32],
	},
	"high": {
		"msaa": Viewport.MSAA_2X, "shadow_atlas": 8192, "soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_HIGH,
		"shadow_splits": 4, "shadow_distance": 1.0, "ssao": true, "ssao_quality": RenderingServer.ENV_SSAO_QUALITY_HIGH,
		"ssil": true, "ssil_quality": RenderingServer.ENV_SSIL_QUALITY_LOW, "volumetric": "weather", "sdfgi": false,
		"glow": true, "fog_grid": [64, 48],
	},
	"ultra": {
		"msaa": Viewport.MSAA_4X, "shadow_atlas": 8192, "soft_shadows": RenderingServer.SHADOW_QUALITY_SOFT_ULTRA,
		"shadow_splits": 4, "shadow_distance": 1.35, "ssao": true, "ssao_quality": RenderingServer.ENV_SSAO_QUALITY_ULTRA,
		"ssil": true, "ssil_quality": RenderingServer.ENV_SSIL_QUALITY_HIGH, "volumetric": "always", "sdfgi": true,
		"glow": true, "fog_grid": [128, 64],
	},
}


## Niveau imposé par le banc A/B (`BattleScene`, `--bench-ab=`), "" sinon.
static var override_level: String = ""


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
	RenderingServer.directional_shadow_atlas_set_size(int(p["shadow_atlas"]), true)
	RenderingServer.directional_soft_shadow_filter_set_quality(p["soft_shadows"])
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


## Enregistre un environnement (et son soleil) : il sera réappliqué quand le réglage change.
## `context` : "battle" ou "campaign" ; `weather` : clé météo de la bataille ("" en campagne).
static func register(world_env: WorldEnvironment, sun: DirectionalLight3D, context: String, weather: String = "") -> void:
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


## Réapplique le niveau courant à tous les environnements enregistrés (réglage modifié).
static func reapply(tree: SceneTree) -> void:
	apply_global(tree.root)
	for node in tree.get_nodes_in_group(GROUP):
		if node is WorldEnvironment:
			apply_environment(node)
