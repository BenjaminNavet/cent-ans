class_name DiplomacyOffersSection
extends DiplomacyView

## « Propositions reçues » en tête de l'écran de diplomatie : une rangée par offre (réponse
## acceptée ou refusée, appels féodaux d'arbitrage avec choix du parti).

signal offer_answered(offer_id: int, accept: bool)
signal arbitration_requested(offer_id: int, verdict: String, side: String)


func _init() -> void:
	super("Offers", 4, false)


func _render() -> void:
	UiBuild.clear_children(self)
	var offers: Array = sim.call("get_offers")
	if offers.is_empty():
		return
	add_child(_section("Propositions reçues"))
	var feudal := {}  # FE6 : appels féodaux (protection, arbitrage) par id d'offre
	if sim.has_method("get_feudal_offers"):
		for call in sim.call("get_feudal_offers"):
			feudal[int(call.get("id", -1))] = call
	for offer in offers:
		var row := UiBuild.hbox(6)
		var heraldry := TextureRect.new()
		heraldry.texture = PortraitLoader.heraldry_texture(str(offer.get("from", "")))
		heraldry.custom_minimum_size = Vector2(26, 26)
		heraldry.expand_mode = TextureRect.EXPAND_IGNORE_SIZE
		heraldry.stretch_mode = TextureRect.STRETCH_KEEP_ASPECT_CENTERED
		row.add_child(heraldry)
		var text := _label("%s (%s)" % [str(offer["text"]), FrText.count(int(offer["expires_in"]), "tour", "tours")], UiType.BODY, HudStyle.INK)
		text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		row.add_child(text)
		var offer_id := int(offer["id"])
		var kind := str(offer.get("kind", ""))
		var yes := UiBuild.button({"protection": "Intervenir", "arbitration": "Imposer la paix", "summons": "Obéir"}.get(kind, "Accepter"), func() -> void: offer_answered.emit(offer_id, true), row)
		if kind == "arbitration" and feudal.has(offer_id):
			var call: Dictionary = feudal[offer_id]
			for side in [["attacker", "attacker_name"], ["target", "target_name"]]:
				var side_id := str(call.get(side[0], ""))
				var take := UiBuild.button("Soutenir %s" % str(call.get(side[1], side_id)))
				TooltipHost.attach_plain(take, "feudal_take_side", {"title": "Prendre le parti de %s" % str(call.get(side[1], side_id)), "body": "Guerre contre l'autre vassal."})
				take.pressed.connect(func() -> void: arbitration_requested.emit(offer_id, "take_side", side_id))
				row.add_child(take)
		var no := UiBuild.button({"protection": "Se dérober", "arbitration": "Laisser faire", "summons": "Passer outre"}.get(kind, "Refuser"), func() -> void: offer_answered.emit(offer_id, false), row)
		add_child(row)
