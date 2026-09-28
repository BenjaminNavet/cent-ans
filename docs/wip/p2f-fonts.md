# P2f — `add_theme_font_size_override` restants → `UiType`

Lot phase 2 de PO (polish). Voir `docs/wip/po.md` (case P2f) et la bible DA § 12.1/§12.2
(échelle `UiType` : `TITLE`=26, `HEADING`=20, `BODY`=17, `CAPTION`=14).

## But

Remplacer les `add_theme_font_size_override(..., <nombre en dur>)` restants par
`UiType.size(UiType.<Variation>)`, sauf exclusions (autres sessions en cours) et cas
légitimes (taille calculée dynamiquement, rendu 3D/monde, déjà migré).

## Périmètre

- `grep -rln add_theme_font_size_override game/scripts` : 44 fichiers, 111 occurrences.
- Exclus (sessions parallèles, non touchés) : `battle/battle_siege.gd`,
  `battle/battle_alerts_column.gd`, `battle/siege_capture_points.gd`, `ui/rich_tooltip.gd`,
  `ui/tooltip_view.gd`, `ui/plain_tooltip_host.gd`, `ui/encyclopedia.gd`, `map/stance_bar.gd`
  (aucun `plain_tooltip_host.gd`/`stance_bar.gd` trouvé dans le dépôt actuel — rien à exclure
  en pratique pour ces deux-là, seuls `rich_tooltip.gd`, `tooltip_view.gd`, `encyclopedia.gd`
  existaient dans la liste et ont été retirés).
- `game/scripts/ui/ui_type.gd` exclu : c'est l'infrastructure elle-même (définit `UiType.size`),
  pas un appel à migrer.
- Reste : 40 fichiers, 105 occurrences.

## Tri des 105 occurrences

- **74 occurrences** dans 34 fichiers passaient déjà un nombre littéral en argument →
  remplacées par `UiType.size(UiType.<Variation>)`. Règle mécanique : la variation la plus
  proche de la taille de base d'origine (`TITLE`=26, `HEADING`=20, `BODY`=17, `CAPTION`=14),
  départagée par le rôle du libellé quand la distance était égale ou ambiguë (titre de
  fenêtre/panneau → `HEADING`/`TITLE`, texte courant → `BODY`, libellé secondaire/meta/erreur
  → `CAPTION`). Détail ligne par ligne dans le diff de la branche.
- **1 occurrence laissée en l'état, avec commentaire** :
  `game/scripts/map/victory_controller.gd:94` (`end_title`, 40 px) — bannière de fin de partie
  (« Victoire »/« Défaite »), volontairement hors des 4 paliers pour l'effet dramatique de
  l'écran de fin. Aucun palier `UiType` ne convient (le plus proche, `TITLE`=26, casserait
  l'effet). Commentaire ajouté dans le code.
- **30 occurrences** dans les mêmes fichiers passaient déjà une variable/expression (pas un
  nombre en dur) : paramètres de fonctions réutilisables (`battle_hud._label(text, size)`,
  `battle_ui_kit.label_font/button_font`, `front_end_style`, `hud_style.apply_font_size`,
  `outcome_band`, `budget_table`, `icon_chip`, `edict_section`/`coinage_section`/
  `table_section` — lignes qui appliquent un `font_size` reçu en paramètre), constantes
  locales (`army_markers.PLATE_FONT_SIZE`, `HudStyle.FONT_SMALL`/`FONT_BODY`), tailles
  calculées (`portrait_frame` : `size.y * 0.36`), déjà migrées vers `UiType`
  (`codex_bubbles.gd`, `codex_window.gd`, `chivalry_section.gd`, `credits_screen.gd`,
  `character_sheet.gd`), ou l'application de l'échelle utilisateur elle-même
  (`settings.gd:328`, mécanisme `UiType`/`Settings` lui-même). Non touchées : le lot ne porte
  que sur les nombres en dur dans l'appel `add_theme_font_size_override`, pas sur les
  définitions de constantes ou de paramètres par défaut des fonctions utilitaires — un
  éventuel nettoyage de ces constantes (`HudStyle.FONT_SMALL` etc.) serait un lot à part
  (refonte, hors du périmètre « lignes de taille de police seules » de P2f).

## Tests

- `smoke.gd`
- `po_ui_test.gd` (C3 : 4 tailles seulement)
- `p2c_ui_test.gd`, `p2g_ui_test.gd`
- Tests existants des écrans touchés (voir `ls game/tests`)

## État

- [x] Squelette (ce fichier), périmètre et tri déterminés.
- [x] 74 remplacements mécaniques appliqués (script Python, édition par ligne exacte).
- [x] Commentaire ajouté pour l'exception `victory_controller.gd`.
- [x] Tests : `smoke.gd` OK (29 points « smoke OK », aucun FAIL), `po_ui_test.gd` OK
      (C3 = [14, 17, 20, 26]), `p2c_ui_test.gd` OK (C3 = [14, 17, 26] — inchangé, fichiers
      exclus), `p2g_ui_test.gd` OK (C3 = [14, 17, 20, 26]), `po_grade_test.gd` OK (campagne
      4 saisons + bataille 48 contextes). `chronicle_screenshot.gd`/`coinage_screenshot.gd`
      non lancés : scripts à fenêtre, exclus par consigne.
- [x] Terminé, prêt pour le rapport.

## Prochaine étape

Rien : lot terminé. Reste à fusionner (hors périmètre de cet agent — l'orchestrateur PO
fusionnera `feat/p2f-fonts` comme les autres lots P2).
