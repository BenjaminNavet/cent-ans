class_name ConfirmDialog
extends RefCounted

## Façade des confirmations oui / non : un `ConfirmPanel` jetable, ouvert dans `parent`.


## Dialogue jetable : s'ouvre dans `parent`, appelle `on_yes` si confirmé, puis se libère.
static func ask(parent: Node, title: String, text: String, on_yes: Callable, yes_label := "Confirmer", no_label := "Renoncer") -> ConfirmPanel:
	var dialog := ConfirmPanel.new(yes_label, no_label)
	dialog.confirmed.connect(on_yes)
	dialog.confirmed.connect(dialog.queue_free)
	dialog.cancelled.connect(dialog.queue_free)
	parent.add_child(dialog)
	dialog.open(title, text)
	return dialog
