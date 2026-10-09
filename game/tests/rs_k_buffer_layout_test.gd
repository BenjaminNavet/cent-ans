extends TestCase

## Test du lot RS-K (rendu réel requis : sans `--headless`, le rendu factice ne garde pas les
## tampons) : `MapInstancing.write_transform` / `write_custom` écrivent les tampons MultiMesh au
## même format que `set_instance_transform` / `set_instance_custom_data` (avec et sans données
## d'instance). Sauté en headless.
## Usage : godot --path game --script res://tests/rs_k_buffer_layout_test.gd


func _init() -> void:
	await process_frame
	if DisplayServer.get_name() == "headless":
		print("rs_k_buffer_layout_test: headless renderer keeps no buffers, skipped")
	else:
		_run(true)
		_run(false)
	finish()


func _run(custom: bool) -> void:
	var rng := RandomNumberGenerator.new()
	rng.seed = 7
	var count := 5
	var stride := 16 if custom else 12
	var reference := MultiMesh.new()
	reference.transform_format = MultiMesh.TRANSFORM_3D
	reference.use_custom_data = custom
	reference.mesh = QuadMesh.new()
	reference.instance_count = count
	var packed := PackedFloat32Array()
	packed.resize(count * stride)
	for n in count:
		var basis := Basis(Vector3(rng.randf(), rng.randf(), rng.randf()), Vector3(rng.randf(), rng.randf(), rng.randf()), Vector3(rng.randf(), rng.randf(), rng.randf()))
		var xform := Transform3D(basis, Vector3(rng.randf() * 100.0, rng.randf(), rng.randf() * 100.0))
		var color := Color(rng.randf(), rng.randf(), rng.randf(), rng.randf())
		reference.set_instance_transform(n, xform)
		MapInstancing.write_transform(packed, n * stride, xform)
		if custom:
			reference.set_instance_custom_data(n, color)
			MapInstancing.write_custom(packed, n * stride + 12, color)
	var expected := reference.buffer
	if expected.size() != packed.size():
		failures += 1
		push_error("rs_k_buffer_layout_test: size %d != %d (custom %s)" % [packed.size(), expected.size(), custom])
		return
	for k in packed.size():
		if absf(packed[k] - expected[k]) > 1e-5:
			failures += 1
			push_error("rs_k_buffer_layout_test: float %d differs (custom %s): %f != %f" % [k, custom, packed[k], expected[k]])
			return
	print("rs_k_buffer_layout_test: %d floats identical (custom %s)" % [packed.size(), custom])
