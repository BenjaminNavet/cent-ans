class_name CodexText
extends RefCounted

## Mots « découverte » (H2) : convertit les liens `[[cdx_id]]` / `[[cdx_id|libellé]]` des
## textes de `data/` en BBCode `[url=cdx:id]` rubriqué — encre rouge soulignée tant que la fiche
## n'a pas été lue, encre brune ensuite. `auto_link` lie en plus la première occurrence (mot
## entier, casse ignorée) de chaque alias du Codex dans les textes produits par la simulation
## (chronique, messages), hors balises et hors liens existants.
##
## B8 homonymes : à une même position, l'alias le plus long gagne ; une fiche peut lister des
## `exclude_contexts` (« Louis de Poitiers » ne lie pas la bataille) ; `[[!texte]]` rend `texte`
## en texte simple, jamais auto-lié (échappement explicite).
##
## `CodexStore` est obtenu par `/root/CodexStore` (le smoke `--script` est compilé avant les
## autoloads). Sans lui, les liens sont rendus en texte simple.

const META_PREFIX := "cdx:"
## IB4 (ADR 0109) : liens vers une infobulle riche d'entité ou une bulle de règle,
## `[url=ib:<kind>:<id>]` (`ib:unit:unit_longbowmen`, `ib:rule:army_morale`), ouverts par `CodexBubbles`.
const IB_PREFIX := "ib:"
const UNREAD_COLOR := "#8b1a1a"
const READ_COLOR := "#5a3a1a"
## Préfixe d'échappement : `[[!Louis de Poitiers]]` = texte simple, sans auto-lien.
const ESCAPE_PREFIX := "!"
## Balise neutre (langue du texte) qui protège un passage échappé de l'auto-lien, y compris si
## le BBCode repasse par `format`.
const NO_LINK_OPEN := "[lang=fr]"
const NO_LINK_CLOSE := "[/lang]"

static var _link_regex: RegEx
## IB4 : clés `ib:` déjà ouvertes en bulle pendant la session (lien non souligné ensuite).
static var _seen_ib: Dictionary = {}
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


## IB4 : BBCode d'un lien `ib:<kind>:<id>` affiché `label`, en couleur de mot-clé ; souligné tant
## que la bulle n'a pas été ouverte (entité : tant que sa fiche du Codex n'est pas lue).
static func ib_link(kind: String, id: String, label: String) -> String:
	if label == "" or id == "":
		return label
	var key := "%s%s:%s" % [IB_PREFIX, kind, id]
	if is_ib_read(key):
		return "[url=%s][color=%s]%s[/color][/url]" % [key, READ_COLOR, label]
	return "[url=%s][color=%s][u]%s[/u][/color][/url]" % [key, UNREAD_COLOR, label]


## IB4 : clé « ib:<kind>:<id> » désignée par une méta de `RichTextLabel`, vide sinon.
static func ib_key(meta: Variant) -> String:
	var value := str(meta)
	return value if value.begins_with(IB_PREFIX) and value.count(":") >= 2 else ""


## IB4 : note qu'une bulle `ib:` a été ouverte (le lien perd son soulignement).
static func mark_ib_read(key: String) -> void:
	_seen_ib[key] = true


## IB4 : vrai si la bulle `key` a été ouverte, ou si l'entité a une fiche du Codex déjà lue.
static func is_ib_read(key: String) -> bool:
	if _seen_ib.has(key):
		return true
	var codex := store()
	if codex == null:
		return false
	var entry := str(codex.call("entry_for_entity", key.get_slice(":", 2)))
	return entry != "" and bool(codex.call("is_discovered", entry))


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
		if match.get_string(1).begins_with(ESCAPE_PREFIX):
			label = _escaped_text(match)
		elif label == "":
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
		if match.get_string(1).begins_with(ESCAPE_PREFIX):
			out += NO_LINK_OPEN + _escaped_text(match) + NO_LINK_CLOSE
		elif codex == null or not bool(codex.call("has_entry", id)):
			out += label if label != "" else (str(codex.call("title", id)) if codex != null else id)
		else:
			out += link(id, label)
			linked[id] = true
		cursor = match.get_end()
	return out + text.substr(cursor)


## Texte d'un échappement `[[!texte]]` (un éventuel `|…` en fait partie).
static func _escaped_text(match: RegExMatch) -> String:
	return match.get_string(0).substr(3, match.get_string(0).length() - 5)


## Lie la première occurrence de chaque fiche dans les segments de texte hors balises et hors
## `[url]…[/url]` / `[img]…[/img]` / passages échappés `[lang=fr]…[/lang]` ; une fiche déjà liée (explicitement ou plus haut) ne l'est pas deux fois.
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
		if tag_text.begins_with("[url") or tag_text.begins_with("[img") or tag_text.begins_with("[lang"):
			skip_depth += 1
		elif tag_text == "[/url]" or tag_text == "[/img]" or tag_text == "[/lang]":
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
		if bool(codex.call("is_excluded", id, segment, match.get_start(1), match.get_end(1))):
			continue  # homonyme : « Louis de Poitiers » n'est pas la bataille
		linked[id] = true
		out += segment.substr(cursor, match.get_start() - cursor) + link(id, match.get_string(1))
		cursor = match.get_end()
	return out + segment.substr(cursor)
