class_name RuinMarkers
extends ScreenSigns

## Signe de ruine sur la carte de campagne pour une colonie rasée (`get_ruined_places`,
## cœur : `captures.ruins`). Réutilise l'icône existante `delapouite-castle-ruins` en sprite
## toujours de face, de taille constante à l'écran. Rendu seulement ; reconstruit quand la liste
## des ruines change. Sans la méthode du pont (simulation factice) : aucun signe.

const ICON := "res://assets/icons/delapouite-castle-ruins.svg"
## Largeur à l'écran en fraction de la distance caméra.
@export var screen_fraction: float = 0.035
@export var lift: float = 5.0

var _signature := ""


## `sim` : CampaignSim ; `position_of(id) -> Vector3` : position monde de la colonie.
func refresh(sim: Object, position_of: Callable) -> void:
	var ruins: Array = sim.call("get_ruined_places") if sim != null and sim.has_method("get_ruined_places") else []
	var signature := ",".join(PackedStringArray(ruins.map(func(r: Dictionary) -> String: return str(r.get("id", "")))))
	if signature == _signature and _signs.size() == ruins.size():
		return
	_signature = signature
	_clear_signs()
	var texture: Texture2D = load(ICON) as Texture2D
	if texture == null:
		return
	for ruin: Dictionary in ruins:
		var at: Vector3 = position_of.call(str(ruin.get("id", "")))
		if at == Vector3.ZERO:
			continue
		var sprite := Sprite3D.new()
		sprite.name = "Ruin_" + str(ruin.get("id", ""))
		sprite.texture = texture
		sprite.billboard = BaseMaterial3D.BILLBOARD_ENABLED
		sprite.shaded = false
		sprite.double_sided = true
		sprite.modulate = Color(0.2, 0.15, 0.11, 0.95)
		sprite.no_depth_test = true
		sprite.position = at + Vector3(0.0, lift, 0.0)
		sprite.set_meta("turns_left", int(ruin.get("turns_left", 0)))
		_add_sign(sprite)
	_rescale()


func _process(_delta: float) -> void:
	if not _signs.is_empty():
		_rescale()


func _apply_size(sign_node: Node3D, distance: float) -> void:
	var sprite := sign_node as Sprite3D
	sprite.pixel_size = maxf(distance, 0.01) * screen_fraction / maxf(float(sprite.texture.get_width()), 1.0)
