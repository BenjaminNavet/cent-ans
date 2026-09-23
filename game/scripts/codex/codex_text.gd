class_name CodexText
extends RefCounted

## Mots « découverte » (H2) : convertit les liens `[[cdx_id]]` / `[[cdx_id|libellé]]` des
## textes de `data/` en BBCode `[url=cdx:id]` rubriqué — encre rouge soulignée tant que la fiche
## n'a pas été lue, encre brune ensuite. `auto_link` lie en plus la première occurrence (mot
## entier, casse ignorée) de chaque alias du Codex dans les textes produits par la simulation
## (chronique, messages), hors balises et hors liens existants.
##
## `CodexStore` est obtenu par `/root/CodexStore` (le smoke `--script` est compilé avant les
## autoloads). Sans lui, les liens sont rendus en texte simple.

const META_PREFIX := "cdx:"
const UNREAD_COLOR := "#8b1a1a"
const READ_COLOR := "#5a3a1a"

static var _link_regex: RegEx
static var _tag_regex: RegEx


static func store() -> Node:
	var loop := Engine.get_main_loop() as SceneTree
	return loop.root.get_node_or_null("/root/CodexStore") if loop != null else null


## Texte BBCode avec les liens du Codex ; `auto_link` : alias reconnus dans le texte libre.
static func format(text: String, auto_link: bool = false) -> String:
	if text == "":
		return text
	var codex := store()
	var linked := {}
	var out := _replace_links(text, codex, linked)
	if auto_link and codex != null:
		out = _auto_link(out, codex, linked)
	return out


## BBCode d'un lien vers `id` affiché `label` (titre de la fiche si vide).
static func link(id: String, label: String = "") -> String:
	var codex := store()
	if codex == null or not bool(codex.call("has_entry", id)):
		return label if label != "" else id
	if label == "":
		label = str(codex.call("title", id))
	if bool(codex.call("is_discovered", id)):
		return "[url=%s%s][color=%s]%s[/color][/url]" % [META_PREFIX, id, READ_COLOR, label]
	return "[url=%s%s][color=%s][u]%s[/u][/color][/url]" % [META_PREFIX, id, UNREAD_COLOR, label]


## Id du Codex désigné par une méta de `RichTextLabel` (`cdx:…`), vide sinon.
static func meta_id(meta: Variant) -> String:
	var value := str(meta)
	return value.trim_prefix(META_PREFIX) if value.begins_with(META_PREFIX) else ""


## Retire le balisage `[[…]]` (libellé seul) : pour les textes affichés hors BBCode.
static func plain(text: String) -> String:
	var codex := store()
	var out := ""
	var cursor := 0
	for match in _links().search_all(text):
		out += text.substr(cursor, match.get_start() - cursor)
		var label := match.get_string(2)
		if label == "":
			label = str(codex.call("title", match.get_string(1).strip_edges())) if codex != null else match.get_string(1)
		out += label
		cursor = match.get_end()
	return out + text.substr(cursor)


static func _links() -> RegEx:
	if _link_regex == null:
		_link_regex = RegEx.create_from_string("\\[\\[([^\\]|]+)(?:\\|([^\\]]*))?\\]\\]")
	return _link_regex


static func _tags() -> RegEx:
	if _tag_regex == null:
		_tag_regex = RegEx.create_from_string("\\[[^\\[\\]]*\\]")
	return _tag_regex


static func _replace_links(text: String, codex: Node, linked: Dictionary) -> String:
	var out := ""
	var cursor := 0
	for match in _links().search_all(text):
		out += text.substr(cursor, match.get_start() - cursor)
		var id := match.get_string(1).strip_edges()
		var label := match.get_string(2).strip_edges()
		if codex == null or not bool(codex.call("has_entry", id)):
			out += label if label != "" else (str(codex.call("title", id)) if codex != null else id)
		else:
			out += link(id, label)
			linked[id] = true
		cursor = match.get_end()
	return out + text.substr(cursor)


## Lie la première occurrence de chaque fiche dans les segments de texte hors balises et hors
## `[url]…[/url]` / `[img]…[/img]` ; une fiche déjà liée (explicitement ou plus haut) ne l'est pas deux fois.
static func _auto_link(bbcode: String, codex: Node, linked: Dictionary) -> String:
	var regex: RegEx = codex.call("alias_regex")
	if regex == null:
		return bbcode
	var out := ""
	var cursor := 0
	var skip_depth := 0
	var tags := _tags().search_all(bbcode)
	tags.append(null)  # sentinelle : dernier segment de texte
	for tag in tags:
		var end: int = tag.get_start() if tag != null else bbcode.length()
		var segment := bbcode.substr(cursor, end - cursor)
		out += segment if skip_depth > 0 else _link_segment(segment, regex, codex, linked)
		if tag == null:
			break
		var tag_text: String = tag.get_string()
		if tag_text.begins_with("[url") or tag_text.begins_with("[img"):
			skip_depth += 1
		elif tag_text == "[/url]" or tag_text == "[/img]":
			skip_depth = maxi(0, skip_depth - 1)
		out += tag_text
		cursor = tag.get_end()
	return out


static func _link_segment(segment: String, regex: RegEx, codex: Node, linked: Dictionary) -> String:
	if segment.strip_edges() == "":
		return segment
	var out := ""
	var cursor := 0
	for match in regex.search_all(segment):
		var id := str(codex.call("id_for_alias", match.get_string(1)))
		if id == "" or linked.has(id):
			continue
		linked[id] = true
		out += segment.substr(cursor, match.get_start() - cursor) + link(id, match.get_string(1))
		cursor = match.get_end()
	return out + segment.substr(cursor)
