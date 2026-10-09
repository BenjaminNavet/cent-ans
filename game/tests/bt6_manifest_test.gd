extends TestCase

## Lot SC bt6 : le manifeste skinné cuit (`manifest_merged.json`, outil
## `tools/cent_ans_tools/bake_skinned_manifest.py`) est celui que `BattleSkinned.manifest()` charge.


func _init() -> void:
	var text := FileAccess.get_file_as_string("res://assets/models/battle_skinned/manifest_merged.json")
	var baked = JSON.parse_string(text)
	check(baked is Dictionary, "manifeste cuit lisible")
	var runtime := BattleSkinned.manifest()
	check(not runtime.is_empty(), "manifeste chargé")
	check(_same(runtime, baked, ""), "manifeste chargé == manifeste cuit")
	var human: Dictionary = (runtime["rigs"] as Dictionary)["fine_human"]
	check((human.get("mocap_textures", []) as Array).size() == 2, "couches melee + FA3")
	finish()


## Égalité profonde ; entiers et flottants égaux par valeur (JSON). Signale la première différence.
func _same(a: Variant, b: Variant, path: String) -> bool:
	if a is Dictionary and b is Dictionary:
		var da: Dictionary = a
		var db: Dictionary = b
		if da.size() != db.size():
			print("taille différente en ", path)
			return false
		for key in da:
			if not db.has(key) or not _same(da[key], db[key], path + "/" + str(key)):
				if not db.has(key):
					print("clé absente ", path, "/", key)
				return false
		return true
	if a is Array and b is Array:
		if a.size() != b.size():
			print("longueur différente en ", path)
			return false
		for i in a.size():
			if not _same(a[i], b[i], path + "[%d]" % i):
				return false
		return true
	if (a is int or a is float) and (b is int or b is float):
		if not is_equal_approx(float(a), float(b)):
			print("valeur différente en ", path, ": ", a, " vs ", b)
			return false
		return true
	if a != b:
		print("valeur différente en ", path, ": ", a, " vs ", b)
		return false
	return true
