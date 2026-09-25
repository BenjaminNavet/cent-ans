extends SceneTree

## Sonde (fenêtrée) : vrais mouvements de souris (warp + push_input) sur des liens du Codex.


func _init() -> void:
	await process_frame
	var store: Node = root.get_node("/root/CodexStore")
	var bubbles: Node = root.get_node("/root/CodexBubbles")
	store.call("use_test_file")
	var label := RichTextLabel.new()
	label.bbcode_enabled = true
	label.fit_content = true
	label.custom_minimum_size = Vector2(400, 0)
	label.position = Vector2(40, 40)
	label.text = CodexText.format("[b]Le roi [[cdx_edouard_iii]][/b] débarque.", true)
	root.add_child(label)
	bubbles.call("attach", label)
	await create_timer(1.5).timeout
	var opened := await _scan(bubbles, label)
	print("probe level1 opened=", opened, " count=", bubbles.call("bubble_count"))
	bubbles.call("close_all")
	await process_frame
	var bubble: PanelContainer = bubbles.call("open", "cdx_crecy", Vector2(600, 300), -1, true)
	for _i in 5:
		await process_frame
	var inner := bubble.find_child("Text", true, false) as RichTextLabel
	print("bubble rect ", bubble.get_global_rect(), " links=", inner.text.count("[url="))
	var nested := await _scan(bubbles, inner)
	print("probe level2 opened=", nested, " count=", bubbles.call("bubble_count"))
	quit(0)


func _scan(bubbles: Node, label: RichTextLabel) -> bool:
	var start: int = bubbles.call("bubble_count")
	var rect := label.get_global_rect()
	var y := rect.position.y + 4
	while y < rect.end.y:
		var x := rect.position.x + 2
		while x < rect.end.x:
			root.warp_mouse(Vector2(x, y))
			var motion := InputEventMouseMotion.new()
			motion.position = Vector2(x, y)
			motion.global_position = Vector2(x, y)
			root.push_input(motion)
			await process_frame
			if str(bubbles.get("_hover_id")) != "":
				var src: Node = bubbles.get("_hover_source")
				print("  hover ", Vector2(x, y), " id=", bubbles.get("_hover_id"), " src=", src.get_path())
				for _k in 40:
					await process_frame
				print("  after: hover=", bubbles.get("_hover_id"), " count=", bubbles.call("bubble_count"), " top=", bubbles.call("top_id"))
				return int(bubbles.call("bubble_count")) > start
			x += 6
		y += 6
	return false
