class_name PanelStack
extends RefCounted

## Pile des panneaux de la carte de campagne (audit A3, lot U1 « Empilement des fenêtres »).
##
## Chaque panneau enregistré a un genre :
## - `DOCKED` : panneaux ancrés à droite (province, colonie). Un panneau central les « met de
##   côté » (masqués, retenus) et ils reviennent quand le dernier panneau central se ferme ;
## - `CENTRAL` : grandes fenêtres (faction, Cour, technologies, diplomatie, objectifs, aide…),
##   exclusives : en ouvrir une ferme les autres ;
## - `COMPANION` : fenêtre qui accompagne un panneau central (fiche de personnage à côté de la
##   Cour, rançons à côté de la faction). Elle ferme les panneaux centraux qu'elle n'accompagne
##   pas, et se ferme avec ceux qu'elle accompagne ;
## - `MODAL` : menus pause, réglages, rapport de saison, confirmations. Tant qu'un modal est
##   ouvert, Échap lui appartient (`close_top` rend faux).
##
## Échap ferme le panneau du dessus (le dernier ouvert). Fermer un panneau = le masquer puis
## émettre son signal `closed` s'il en a un (le propriétaire met son état à jour : province
## désélectionnée, etc.). Aucune règle de jeu ici : ce n'est que de l'agencement.

enum Kind { DOCKED, CENTRAL, COMPANION, MODAL }

## Émis quand l'ensemble des panneaux visibles change (le HUD se replace).
signal changed

## Panneau → `{kind, companion_of: Array[Control]}`.
var _entries: Dictionary = {}
## Panneaux visibles, du plus ancien au plus récent (le dernier est « au-dessus »).
var _order: Array[Control] = []
## Panneaux ancrés mis de côté par un panneau central.
var _suspended: Array[Control] = []
## Vrai pendant une opération de la pile : ses propres `show` / `hide` ne relancent pas les règles.
var _busy := false


## Enregistre `panel` (idempotent). `companion_of` : panneaux centraux qu'un `COMPANION`
## accompagne.
func register(panel: Control, kind: Kind, companion_of: Array = []) -> void:
	if panel == null or _entries.has(panel):
		return
	var companions: Array[Control] = []
	for other in companion_of:
		if other is Control:
			companions.append(other)
	_entries[panel] = {"kind": kind, "companion_of": companions}
	panel.visibility_changed.connect(_on_visibility_changed.bind(panel))
	panel.tree_exiting.connect(_forget.bind(panel))
	if panel.visible:
		_order.append(panel)


func is_registered(panel: Control) -> bool:
	return _entries.has(panel)


func kind_of(panel: Control) -> int:
	return int(_entries[panel]["kind"]) if _entries.has(panel) else -1


## Panneaux visibles, du plus ancien au plus récent.
func visible_panels() -> Array[Control]:
	return _order.duplicate()


## Panneau du dessus (`null` si aucun).
func top() -> Control:
	return _order.back() if not _order.is_empty() else null


func is_suspended(panel: Control) -> bool:
	return _suspended.has(panel)


## Vrai si un panneau central ou compagnon est ouvert (les ancrés sont alors mis de côté).
func has_central_open() -> bool:
	for panel in _order:
		var kind := kind_of(panel)
		if kind == Kind.CENTRAL or kind == Kind.COMPANION:
			return true
	return false


func has_modal_open() -> bool:
	for panel in _order:
		if kind_of(panel) == Kind.MODAL:
			return true
	return false


## Échap : ferme le panneau du dessus. Faux si rien n'est ouvert ou si un modal a la main.
func close_top() -> bool:
	if has_modal_open():
		return false
	var panel := top()
	if panel == null:
		return false
	close(panel)
	return true


## Ferme `panel` : masqué, puis signal `closed` (le propriétaire met son état à jour).
func close(panel: Control) -> void:
	if panel == null or not is_instance_valid(panel):
		return
	_suspended.erase(panel)
	panel.hide()
	if panel.has_signal("closed"):
		panel.emit_signal("closed")


## Un panneau ancré est demandé pour une **nouvelle** sélection (autre province) pendant qu'un
## panneau central est ouvert : on referme les centraux et on oublie les ancrés mis de côté.
func reveal(panel: Control) -> void:
	if not _suspended.has(panel) and not has_central_open():
		return
	_suspended.clear()
	for other in _order.duplicate():
		var kind := kind_of(other)
		if kind == Kind.CENTRAL or kind == Kind.COMPANION:
			close(other)


## Le propriétaire ferme un panneau ancré mis de côté (désélection) : il ne reviendra pas.
func forget_suspended(panel: Control) -> void:
	_suspended.erase(panel)


func _forget(panel: Control) -> void:
	_entries.erase(panel)
	_order.erase(panel)
	_suspended.erase(panel)


func _on_visibility_changed(panel: Control) -> void:
	if not is_instance_valid(panel):
		return
	if panel.visible:
		_order.erase(panel)
		_order.append(panel)
		if not _busy:
			_on_shown(panel)
	else:
		_order.erase(panel)
		if not _busy:
			_on_hidden(panel)
	changed.emit()


func _on_shown(panel: Control) -> void:
	var kind := kind_of(panel)
	_busy = true
	match kind:
		Kind.DOCKED:
			if has_central_open():
				# Rafraîchissement d'un panneau mis de côté : il reste de côté.
				if not _suspended.has(panel):
					_suspended.append(panel)
				panel.hide()
				_order.erase(panel)
		Kind.CENTRAL:
			for other in _order.duplicate():
				if other == panel:
					continue
				var other_kind := kind_of(other)
				if other_kind == Kind.CENTRAL:
					_close_quiet(other)
				elif other_kind == Kind.COMPANION and not (_entries[other]["companion_of"] as Array).has(panel):
					_close_quiet(other)
			_suspend_docked()
		Kind.COMPANION:
			var hosts: Array = _entries[panel]["companion_of"]
			for other in _order.duplicate():
				if other == panel:
					continue
				var other_kind := kind_of(other)
				if other_kind == Kind.CENTRAL and not hosts.has(other):
					_close_quiet(other)
				elif other_kind == Kind.COMPANION:
					_close_quiet(other)
			_suspend_docked()
	_busy = false


func _on_hidden(panel: Control) -> void:
	var kind := kind_of(panel)
	if kind == Kind.CENTRAL:
		# Les compagnons se ferment avec le panneau qu'ils accompagnent.
		_busy = true
		for other in _order.duplicate():
			if kind_of(other) == Kind.COMPANION and (_entries[other]["companion_of"] as Array).has(panel):
				_close_quiet(other)
		_busy = false
	if (kind == Kind.CENTRAL or kind == Kind.COMPANION) and not has_central_open():
		_restore_docked()


func _suspend_docked() -> void:
	for other in _order.duplicate():
		if kind_of(other) == Kind.DOCKED:
			if not _suspended.has(other):
				_suspended.append(other)
			other.hide()
			_order.erase(other)


func _restore_docked() -> void:
	var panels := _suspended.duplicate()
	_suspended.clear()
	_busy = true
	for panel in panels:
		if is_instance_valid(panel):
			panel.show()
	_busy = false


## Fermeture par la pile elle-même (règle d'exclusivité) : masqué + `closed`, sans relancer les
## règles pour ce panneau.
func _close_quiet(panel: Control) -> void:
	_order.erase(panel)
	if not is_instance_valid(panel):
		return
	panel.hide()
	if panel.has_signal("closed"):
		panel.emit_signal("closed")
