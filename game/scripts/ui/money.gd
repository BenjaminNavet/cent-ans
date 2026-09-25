class_name Money
extends RefCounted

## Montants en livres tournois (audit A3 E3, lot U3) : un seul symbole, `₶`, et un seul format
## dans toute l'interface — chiffres groupés par trois (espace insécable), vrai signe moins
## (U+2212), signe explicite pour les soldes et les écarts. Même format que
## `economy_balance::signed_livres` côté `core/`.

const SYMBOL := "₶"
const MINUS := "−"
const NBSP := " "
## Couleurs des montants sur parchemin : perte (rouge sombre), gain (vert sombre), neutre.
const LOSS_COLOR := Color(0.55, 0.12, 0.10)
const GAIN_COLOR := Color(0.18, 0.38, 0.16)
const INK_COLOR := Color(0.22, 0.14, 0.07)


## `60000` → « 60 000 » (sans symbole ; négatif avec le vrai signe moins).
static func digits(value: int) -> String:
	var text := str(absi(value))
	var out := ""
	while text.length() > 3:
		out = NBSP + text.substr(text.length() - 3) + out
		text = text.substr(0, text.length() - 3)
	return (MINUS if value < 0 else "") + text + out


## `60000` → « 60 000 ₶ ».
static func amount(value: int) -> String:
	return digits(value) + NBSP + SYMBOL


## `3063` → « +3 063 ₶ », `-1243` → « −1 243 ₶ », `0` → « 0 ₶ ».
static func signed(value: int) -> String:
	if value == 0:
		return "0" + NBSP + SYMBOL
	return ("+" if value > 0 else MINUS) + digits(absi(value)) + NBSP + SYMBOL


## Charge (valeur positive) → « −9 930 ₶ » ; « 0 ₶ » si nulle.
static func charge(value: int) -> String:
	return MINUS + digits(absi(value)) + NBSP + SYMBOL if value != 0 else "0" + NBSP + SYMBOL


## Couleur d'un montant signé (rouge si négatif, vert si positif, encre si nul).
static func color_of(value: int) -> Color:
	if value < 0:
		return LOSS_COLOR
	return GAIN_COLOR if value > 0 else INK_COLOR


## Écart (flèche et montant signé) : « ▲ +1 200 ₶ », « ▼ −900 ₶ », « = » si nul.
static func delta(value: int) -> String:
	if value == 0:
		return "="
	return ("▲ " if value > 0 else "▼ ") + signed(value)
