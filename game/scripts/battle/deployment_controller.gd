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
## CV3-2 : zones supplémentaires (second flanc d'une embuscade), dessinées comme la première.
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
	# CV3-2 : l'embusqué peut avoir deux zones, une sur chaque flanc de la colonne ennemie.
	var zones: Array = scene.battle.call("get_deployment_zones", scene.player_side) if scene.battle.has_method("get_deployment_zones") else []
	for k in range(1, zones.size()):
		var view := DeploymentZone.new()
		view.name = "DeploymentZone%d" % (k + 1)
		scene.add_child(view)
		view.build(zones[k], height_at)
		extra_views.append(view)
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
	for view in extra_views:
		view.queue_free()
	extra_views.clear()
	banner.queue_free()
	scene.hud.add_events([{"time": 0.0, "text_fr": "La bataille commence."}])
	return true


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
	var out: Array = []
	if ids.is_empty():
		return out
	var line := FormationDrag.drag_line(p0, p1, camera_pos)
	var dragged := bool(line["ok"])
	var facing := float(line["facing"]) if dragged else NAN
	var along := Vector2.ZERO
	var order: Array = ids.duplicate()
	var shares: Array[float] = []
	if dragged:
		along = Vector2(p1.x - p0.x, p1.z - p0.z).normalized()
		# Ordre gauche-droite courant, projeté sur l'axe du glisser.
		var keyed: Array = []
		for k in ids.size():
			var unit := _unit(int(ids[k]))
			keyed.append([Vector2(float(unit.get("x", 0.0)), float(unit.get("z", 0.0))).dot(along), k])
		keyed.sort_custom(func(a: Array, b: Array) -> bool: return a[0] < b[0] or (a[0] == b[0] and a[1] < b[1]))
		order = []
		var counts: Array = []
		for entry in keyed:
			var id := int(ids[int(entry[1])])
			order.append(id)
			counts.append(int(_unit(id).get("soldiers", 1)))
		shares = FormationDrag.split_widths(counts, float(line["width"]), UNIT_GAP)
	else:
		var first := _unit(int(ids[0]))
		var f := float(first.get("facing", 0.0))
		along = Vector2(cos(f), -sin(f))  # perpendiculaire au front (facing = atan2(dx, dz))
	var sizes: Array[Vector2] = []
	var total := 0.0
	for k in order.size():
		var unit := _unit(int(order[k]))
		var size := Vector2(float(unit.get("width", 30.0)), float(unit.get("depth", 6.0)))
		if dragged and scene.battle.has_method("formation_extent"):
			size = scene.battle.call("formation_extent", int(order[k]), shares[k])
		sizes.append(size)
		total += size.x + UNIT_GAP
	total -= UNIT_GAP
	var center := Vector2((p0.x + p1.x) * 0.5, (p0.z + p1.z) * 0.5)
	var cursor := -total * 0.5
	for k in order.size():
		var offset := cursor + sizes[k].x * 0.5
		var target := center + along * offset
		cursor += sizes[k].x + UNIT_GAP
		out.append({"id": int(order[k]), "x": target.x, "z": target.y, "facing": facing if dragged else float(_unit(int(order[k])).get("facing", 0.0)), "share": shares[k] if dragged else 0.0, "width": sizes[k].x, "depth": sizes[k].y})
	if not dragged:
		for spot in out:
			spot["facing"] = NAN
	return out


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
