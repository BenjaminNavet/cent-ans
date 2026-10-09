class_name MultiMeshKit
extends RefCounted
## Fabrique statique de MultiMesh / MultiMeshInstance3D du champ de bataille.
##
## Options (toutes facultatives) :
##   colors (bool)         : `use_colors`
##   custom_data (bool)    : `use_custom_data`
##   count (int)           : `instance_count` (par défaut la taille de `transforms`)
##   visible (int)         : `visible_instance_count`
##   mm_aabb (AABB)        : `custom_aabb` du MultiMesh
##   name (String)         : nom du nœud
##   material (Material)   : `material_override`
##   shadow (bool)         : `cast_shadow` ON si vrai, OFF si faux ; absent = défaut Godot
##   range_begin / range_end / range_end_margin (float) : portée de visibilité
##   fade_self (bool)      : `visibility_range_fade_mode = SELF`
##   aabb (AABB)           : `custom_aabb` de l'instance
##   parent (Node)         : si fourni, l'instance y est ajoutée


## MultiMesh 3D nu : format, couleurs/données d'instance, maillage puis nombre d'instances.
static func make_multimesh(mesh: Mesh, count: int, opts: Dictionary = {}) -> MultiMesh:
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	if opts.get("colors", false):
		mm.use_colors = true
	if opts.get("custom_data", false):
		mm.use_custom_data = true
	mm.mesh = mesh
	mm.instance_count = count
	if opts.has("visible"):
		mm.visible_instance_count = int(opts["visible"])
	if opts.has("mm_aabb"):
		mm.custom_aabb = opts["mm_aabb"]
	return mm


## Remplit `mm` dans l'ordre : transformées, puis couleurs et données d'instance si fournies.
static func fill(mm: MultiMesh, transforms: Array, colors: Array = [], custom_data: Array = []) -> void:
	for i in transforms.size():
		mm.set_instance_transform(i, transforms[i])
	for i in colors.size():
		mm.set_instance_color(i, colors[i])
	for i in custom_data.size():
		mm.set_instance_custom_data(i, custom_data[i])


## Enveloppe `mm` dans un MultiMeshInstance3D réglé selon `opts` (et l'ajoute à `opts.parent`).
static func instance(mm: MultiMesh, opts: Dictionary = {}) -> MultiMeshInstance3D:
	var instance := MultiMeshInstance3D.new()
	if opts.has("name"):
		instance.name = opts["name"]
	instance.multimesh = mm
	if opts.has("material"):
		instance.material_override = opts["material"]
	if opts.has("shadow"):
		instance.cast_shadow = GeometryInstance3D.SHADOW_CASTING_SETTING_ON if opts["shadow"] else GeometryInstance3D.SHADOW_CASTING_SETTING_OFF
	if opts.has("range_begin"):
		instance.visibility_range_begin = float(opts["range_begin"])
	if opts.has("range_end"):
		instance.visibility_range_end = float(opts["range_end"])
	if opts.has("range_end_margin"):
		instance.visibility_range_end_margin = float(opts["range_end_margin"])
	if opts.get("fade_self", false):
		instance.visibility_range_fade_mode = GeometryInstance3D.VISIBILITY_RANGE_FADE_SELF
	if opts.has("aabb"):
		instance.custom_aabb = opts["aabb"]
	var parent: Node = opts.get("parent")
	if parent != null:
		parent.add_child(instance)
	return instance


## MultiMeshInstance3D complet : instances = `transforms` (+ `colors`, `custom_data` parallèles).
static func make(mesh: Mesh, transforms: Array, opts: Dictionary = {}, colors: Array = [], custom_data: Array = []) -> MultiMeshInstance3D:
	var mm := make_multimesh(mesh, int(opts.get("count", transforms.size())), opts)
	fill(mm, transforms, colors, custom_data)
	return instance(mm, opts)
