class_name FrText
extends RefCounted

## Lot U8 (audit A3) : petits outils de français pour l'interface — pluriels exacts au lieu de
## « (s) », dates en toutes lettres (« 24 sept. 2026, 23 h 19 »). Aucune règle de jeu.

const MONTHS_SHORT := ["janv.", "févr.", "mars", "avr.", "mai", "juin", "juil.", "août", "sept.", "oct.", "nov.", "déc."]


## « s » si `count` appelle le pluriel (2 et plus ; 0 et 1 au singulier en français).
static func s(count: int) -> String:
	return "s" if absi(count) > 1 else ""


## « 3 tours », « 1 tour » ; `plural` pour les pluriels irréguliers (« 2 chevaux »).
static func count(value: int, singular: String, plural: String = "") -> String:
	if absi(value) > 1:
		return "%d %s" % [value, plural if plural != "" else singular + "s"]
	return "%d %s" % [value, singular]


## Date et heure en français depuis un horodatage Unix (heure locale) :
## « 24 sept. 2026, 23 h 19 ».
static func datetime(unix_time: int) -> String:
	var bias := int(Time.get_time_zone_from_system().get("bias", 0)) * 60
	var parts := Time.get_datetime_dict_from_unix_time(unix_time + bias)
	return from_dict(parts)


## Même format depuis un dictionnaire `{year, month, day, hour, minute}`, ou une chaîne ISO
## (« 2026-09-24T23:19:05 » ou « 2026-09-24 23:19:05 »).
static func from_dict(parts: Dictionary) -> String:
	var month := clampi(int(parts.get("month", 1)), 1, 12)
	var text := "%d %s %d" % [int(parts.get("day", 1)), MONTHS_SHORT[month - 1], int(parts.get("year", 0))]
	if parts.has("hour"):
		text += ", %d h %02d" % [int(parts.get("hour", 0)), int(parts.get("minute", 0))]
	return text


static func from_iso(text: String) -> String:
	var iso := text.strip_edges().replace(" ", "T")
	if iso.length() < 10:
		return text
	var parts := Time.get_datetime_dict_from_datetime_string(iso, false)
	if int(parts.get("year", 0)) == 0:
		return text
	if not iso.contains("T"):
		parts.erase("hour")
	return from_dict(parts)
