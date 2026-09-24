class_name BattleSiegeBatcher
extends RefCounted

## Regroupe les éléments répétés et statiques d'un siège (maisons, tours) en un petit nombre de
## `MultiMeshInstance3D` (un par maillage unitaire × matière), au lieu d'un nœud par élément
## (V6, perf : ~1170 appels de dessin avant, dont ~550 pour les seules maisons — 46 maisons ×
## ~12 pièces chacune). Purement rendu : ne décide jamais des positions ni de l'apparence (l'appelant
## construit ses éléments normalement, avec sa propre source de positions — tirage aléatoire
## aujourd'hui, obstacles de simulation demain) ; ce script se contente de récupérer les nœuds déjà
## construits sous une racine donnée et de les remplacer par des `MultiMeshInstance3D` strictement
## équivalents.
##
## Limites volontaires : ne traite que les maillages `BoxMesh` et `PrismMesh` (toute la géométrie
## des maisons), dont la taille est un simple facteur d'échelle du maillage — pas les murailles
## (dégâts par pan : brèches, effondrement, porte ouverte/fermée, teinte continue selon les PV,
## incompatible avec un lot figé) ni les tours (troncs de cône `CylinderMesh`, rayon/hauteur non
## réductibles à une échelle générique sans hypothèse supplémentaire par pièce).


## Parcourt `root` (nœud déjà construit, par ex. le sous-groupe des maisons ou des tours),
## regroupe les `MeshInstance3D` (`BoxMesh`/`PrismMesh`) trouvées par (matière, type de maillage)
## en `MultiMeshInstance3D` ajoutés sous `root`. Seules les `MeshInstance3D` effectivement
## récupérées sont libérées ; tout le reste (autre type de maillage — les fûts coniques des
## tours, par ex. — ou pas de matière) reste construit normalement, en toute sécurité : `root`
## peut mélanger du contenu regroupable et du contenu qui ne l'est pas. `root` doit déjà être dans
## l'arbre (positions/rotations calculées via les `transform` locaux, pas besoin de
## `global_transform`).
static func batch_and_replace(root: Node3D) -> void:
	var groups := {}  # clé (matière, classe de maillage) -> {"mesh": Mesh, "transforms": Array[Transform3D]}
	for child in root.get_children():
		if child is Node3D:
			_harvest(child as Node3D, root, groups)
	for key in groups:
		var group: Dictionary = groups[key]
		var transforms: Array = group["transforms"]
		if transforms.is_empty():
			continue
		var mm := MultiMesh.new()
		mm.transform_format = MultiMesh.TRANSFORM_3D
		mm.mesh = group["mesh"]
		mm.instance_count = transforms.size()
		for i in transforms.size():
			mm.set_instance_transform(i, transforms[i])
		var mmi := MultiMeshInstance3D.new()
		mmi.multimesh = mm
		mmi.material_override = key[0]
		root.add_child(mmi)


## Ajoute à `groups` toutes les `MeshInstance3D` de `node` (lui compris) dont le maillage est un
## `BoxMesh`/`PrismMesh`, avec leur transform relatif à `root` (rotation/position du nœud composée
## avec la mise à l'échelle du maillage vers un maillage unitaire équivalent), puis les libère
## individuellement. Toute autre configuration (autre type de maillage, pas de matière) est laissée
## intacte : seuls les nœuds effectivement récupérés disparaissent, jamais leurs voisins.
static func _harvest(node: Node3D, root: Node3D, groups: Dictionary) -> void:
	if node is MeshInstance3D:
		var mesh_instance := node as MeshInstance3D
		var mesh := mesh_instance.mesh
		var material := mesh_instance.material_override
		var mesh_class := ""
		var size := Vector3.ONE
		if mesh is BoxMesh:
			mesh_class = "BoxMesh"
			size = (mesh as BoxMesh).size
		elif mesh is PrismMesh:
			mesh_class = "PrismMesh"
			size = (mesh as PrismMesh).size
		if mesh_class != "" and material != null:
			var relative := _relative_transform(mesh_instance, root)
			var instance_transform := relative * Transform3D(Basis().scaled(size), Vector3.ZERO)
			var key := [material, mesh_class]
			if not groups.has(key):
				var unit: Mesh = BoxMesh.new() if mesh_class == "BoxMesh" else PrismMesh.new()
				if unit is BoxMesh:
					(unit as BoxMesh).size = Vector3.ONE
				else:
					(unit as PrismMesh).size = Vector3.ONE
				groups[key] = {"mesh": unit, "transforms": []}
			(groups[key]["transforms"] as Array).append(instance_transform)
			mesh_instance.queue_free()
	for child in node.get_children():
		if child is Node3D:
			_harvest(child as Node3D, root, groups)


## Transform de `node` relatif à `root` (composition des `transform` locaux ; pas besoin que
## `root` soit la racine de la scène, seulement un ancêtre de `node`).
static func _relative_transform(node: Node3D, root: Node3D) -> Transform3D:
	var t := Transform3D.IDENTITY
	var current: Node = node
	while current != null and current != root:
		if current is Node3D:
			t = (current as Node3D).transform * t
		current = current.get_parent()
	return t
