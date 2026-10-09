class_name DeploymentController
extends Node

## Phase de déploiement d'une bataille du joueur (F5c). La simulation est figée
## (`BattleSim.begin_deployment`) : le joueur déplace ses régiments sélectionnés dans sa zone
## (clic droit : même orientation ; glisser-droit : ligne orientée), puis « Commencer la
## bataille » (ou Entrée) appelle `start_battle`. Toute la règle (zone, refus) est en Rust :
## ce contrôleur ne fait que traduire les clics et afficher les refus.


var scene: Node = null  # BattleScene
var active: bool = false
var zone: Dictionary = {}
var zone_view: DeploymentZone = null
## Zones supplémentaires (second flanc d'une embuscade), dessinées comme la première.
var extra_views: Array[DeploymentZone] = []
var banner: PanelContainer = null


## Ouvre la phase juste après `setup` ; `false` si la simulation la refuse.
func open(p_scene: Node) -> bool:
	scene = p_scene
	if not bool(scene.battle.call("begin_deployment")):
		return false
	active = true
	zone = scene.battle.call("get_deployment_zone", scene.player_side)
	zone_view = DeploymentZone.new()
	zone_view.name = "DeploymentZone"
	scene.add_child(zone_view)
	var height_at := func(x: float, z: float) -> float: return scene.terrain.height_at(x, z)
	zone_view.build(zone, height_at)
	# L'embusqué peut avoir deux zones, une sur chaque flanc de la colonne ennemie.
	var zones: Array = scene.battle.call("get_deployment_zones", scene.player_side)
	for k in range(1, zones.size()):
		var view := DeploymentZone.new()
		view.name = "DeploymentZone%d" % (k + 1)
		scene.add_child(view)
		view.build(zones[k], height_at)
		extra_views.append(view)
	_build_banner()
	frame_zone()
	scene.hud.toast_anchor_y = 0.74  # refus de placement au-dessus des cartes, pas sur les unités
	return true


## RX batvis : caméra de déploiement cadrée sur la zone du joueur (régiments lisibles, zone entière
## à l'écran, regard vers l'ennemi) plutôt que le cadrage d'ouverture générique.
func frame_zone() -> void:
	if zone.is_empty() or scene.camera_rig == null:
		return
	var width := float(zone["x1"]) - float(zone["x0"])
	var depth := float(zone["z1"]) - float(zone["z0"])
	var ahead := 1.0 if scene.player_side == "attacker" else -1.0
	var center := Vector3((float(zone["x0"]) + float(zone["x1"])) * 0.5, 0, (float(zone["z0"]) + float(zone["z1"])) * 0.5)
	center.z += ahead * depth * 0.1
	var distance := clampf(maxf(width * 0.4, depth * 0.8), 110.0, 210.0)
	scene.camera_rig.look_at_point(center, distance, PI if ahead > 0.0 else 0.0)


## Fin de la phase : `start_battle`, zone et bandeau retirés. `true` si la bataille commence.
func finish() -> bool:
	if not active:
		return false
	var result: Dictionary = scene.battle.call("start_battle")
	if not bool(result.get("ok", false)):
		scene.hud.show_toast(str(result.get("error", "?")))
		return false
	dismiss()
	scene.hud.add_events([{"time": 0.0, "text_fr": "La bataille commence."}])
	return true


## Zone et bandeau retirés sans lancer la bataille (« Quitter la bataille » pendant le déploiement).
func dismiss() -> void:
	if not active:
		return
	active = false
	zone_view.queue_free()
	for view in extra_views:
		view.queue_free()
	extra_views.clear()
	banner.queue_free()
	scene.hud.toast_anchor_y = 0.3


## Clic droit (`p0 == p1`) : la sélection se range autour du point, orientation gardée ;
## glisser-droit : répartie sur la ligne p0 → p1, front tourné à l'opposé de la caméra ; CB1 : la
## longueur du glisser donne la largeur totale du front, partagée au prorata des effectifs
## (`FormationDrag.split_widths`), les régiments rangés dans leur ordre gauche-droite le long du
## glisser ; chacun prend la largeur retenue par le cœur (rangs bornés, `formation_extent`).
## Renvoie le nombre de régiments placés (les refus s'affichent en toast).
func place(ids: Array, p0: Vector3, p1: Vector3, camera_pos: Vector3) -> int:
	if not active or ids.is_empty():
		return 0
	var placed := 0
	for spot in plan(ids, p0, p1, camera_pos):
		var result: Dictionary = scene.battle.call("deploy_unit", int(spot["id"]), float(spot["x"]), float(spot["z"]), float(spot["facing"]), float(spot["share"]))
		if bool(result.get("ok", false)):
			placed += 1
		else:
			scene.hud.show_toast(str(result.get("error", "?")))
	return placed


## Places que `place` donnerait : [{id, x, z, facing, share, width, depth}] (`facing` NaN : gardée ;
## `share` : largeur demandée au cœur, 0 sans glisser ; `width`/`depth` : taille retenue).
## Sert aussi aux fantômes du glisser en direct.
func plan(ids: Array, p0: Vector3, p1: Vector3, camera_pos: Vector3) -> Array:
	if ids.is_empty():
		return []
	# Règle dans le cœur (`sim-battle/src/deployment.rs`, SC BT8).
	return scene.battle.call("plan_deployment", PackedInt32Array(ids), p0, p1, camera_pos)


func _unit(id: int) -> Dictionary:
	for unit in scene.units:
		if int(unit["id"]) == id:
			return unit
	return {}


func _build_banner() -> void:
	banner = PanelContainer.new()
	banner.name = "DeploymentBanner"
	banner.anchor_left = 0.5
	banner.anchor_right = 0.5
	banner.offset_left = -260
	banner.offset_right = 260
	banner.offset_top = 104  # sous le bandeau de siège
	scene.hud.root.add_child(banner)
	var box := HBoxContainer.new()
	box.alignment = BoxContainer.ALIGNMENT_CENTER
	box.add_theme_constant_override("separation", 16)
	banner.add_child(box)
	var label := Label.new()
	label.text = "Déploiement — placez vos troupes"
	label.add_theme_font_size_override("font_size", UiType.size(UiType.BODY))
	label.add_theme_color_override("font_color", Color(0.22, 0.14, 0.07))
	box.add_child(label)
	var button := Button.new()
	button.name = "StartButton"
	button.text = "Commencer la bataille (Entrée)"
	button.pressed.connect(finish)
	box.add_child(button)
