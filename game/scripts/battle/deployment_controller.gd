class_name DeploymentController
extends Node

## Phase de déploiement d'une bataille du joueur (F5c). La simulation est figée
## (`BattleSim.begin_deployment`) : le joueur déplace ses régiments sélectionnés dans sa zone
## (clic droit : même orientation ; glisser-droit : ligne orientée), puis « Commencer la
## bataille » (ou Entrée) appelle `start_battle`. Toute la règle (zone, refus) est en Rust :
## ce contrôleur ne fait que traduire les clics et afficher les refus.

const UNIT_GAP := 6.0

var scene: Node = null  # BattleScene
var active: bool = false
var zone: Dictionary = {}
var zone_view: DeploymentZone = null
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
	zone_view.build(zone, func(x: float, z: float) -> float: return scene.terrain.height_at(x, z))
	_build_banner()
	return true


## Fin de la phase : `start_battle`, zone et bandeau retirés. `true` si la bataille commence.
func finish() -> bool:
	if not active:
		return false
	var result: Dictionary = scene.battle.call("start_battle")
	if not bool(result.get("ok", false)):
		scene.hud.show_toast(str(result.get("error", "?")))
		return false
	active = false
	zone_view.queue_free()
	banner.queue_free()
	scene.hud.add_events([{"time": 0.0, "text_fr": "La bataille commence."}])
	return true


## Clic droit (`p0 == p1`) : la sélection se range autour du point, orientation gardée ;
## glisser-droit : répartie sur la ligne p0 → p1, front tourné à l'opposé de la caméra.
## Renvoie le nombre de régiments placés (les refus s'affichent en toast).
func place(ids: Array, p0: Vector3, p1: Vector3, camera_pos: Vector3) -> int:
	if not active or ids.is_empty():
		return 0
	var dir := Vector2(p1.x - p0.x, p1.z - p0.z)
	var facing := NAN
	var along := Vector2.ZERO
	var widths: Array[float] = []
	var total := 0.0
	for id in ids:
		var unit := _unit(int(id))
		widths.append(float(unit.get("width", 30.0)))
		total += widths[-1] + UNIT_GAP
	total -= UNIT_GAP
	if dir.length() >= 2.0:
		var normal := Vector2(-dir.y, dir.x).normalized()
		var mid := (p0 + p1) * 0.5
		if normal.dot(Vector2(mid.x - camera_pos.x, mid.z - camera_pos.z)) < 0.0:
			normal = -normal
		facing = atan2(normal.x, normal.y)
		along = dir.normalized()
		total = maxf(total, dir.length())
	else:
		var first := _unit(int(ids[0]))
		var f := float(first.get("facing", 0.0))
		along = Vector2(cos(f), -sin(f))  # perpendiculaire au front (facing = atan2(dx, dz))
	var center := Vector2((p0.x + p1.x) * 0.5, (p0.z + p1.z) * 0.5)
	var spare := (total - _sum(widths) - UNIT_GAP * (ids.size() - 1)) / maxf(ids.size() - 1, 1)
	var cursor := -total * 0.5
	var placed := 0
	for i in ids.size():
		var offset := cursor + widths[i] * 0.5
		var target := center + along * offset
		cursor += widths[i] + UNIT_GAP + maxf(spare, 0.0)
		var result: Dictionary = scene.battle.call("deploy_unit", int(ids[i]), target.x, target.y, facing)
		if bool(result.get("ok", false)):
			placed += 1
		else:
			scene.hud.show_toast("%s : %s" % [str(_unit(int(ids[i])).get("name", "")), str(result.get("error", "?"))])
	return placed


func _unit(id: int) -> Dictionary:
	for unit in scene.units:
		if int(unit["id"]) == id:
			return unit
	return {}


static func _sum(values: Array[float]) -> float:
	var total := 0.0
	for v in values:
		total += v
	return total


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
	label.add_theme_font_size_override("font_size", 18)
	label.add_theme_color_override("font_color", Color(0.22, 0.14, 0.07))
	box.add_child(label)
	var button := Button.new()
	button.name = "StartButton"
	button.text = "Commencer la bataille (Entrée)"
	button.pressed.connect(finish)
	box.add_child(button)
