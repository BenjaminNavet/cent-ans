# P2a — cour, fiche personnage, arbre familial (fichier de reprise)

Lot Phase 2 du chantier PO (polish), ADR 0097, bible DA § 12. Brief : `docs/wip/po.md` §
« Phase 2 », plan `docs/superpowers/plans/2026-09-27-po-polish.md`.

Fichiers du lot (ne pas en sortir) :
- `game/scripts/ui/court_panel.gd` + `game/scenes/ui/court_panel.tscn`
- `game/scripts/ui/character_sheet.gd` + `game/scenes/ui/character_sheet.tscn`
- `game/scripts/ui/family_tree_view.gd`
- `game/scripts/ui/retinue_row.gd`
- `game/scripts/ui/chivalry_section.gd`
- `game/scripts/ui/skill_tree_view.gd`
- Tests : `game/tests/p2a_ui_test.gd`, `game/tests/p2a_shot.gd`

## Point ouvert (à trancher par l'orchestrateur, pas par ce lot)

`CourtPanel` et `CharacterSheet` sont aujourd'hui des enfants directs de `map_ui.gd`
(`_setup_panel_stack()` : `PanelStack.Kind.CENTRAL` / `COMPANION`), positionnés par
`map_ui.layout_hud()` (`set_max_right`, `fit_beside`, `_keep_on_screen` — condition
`panel.get_parent() == self`). Les réclamer dans une zone `UiLayout` (`claim`/`UiZones.put`)
reparente le contrôle et casse ce mécanisme (compagnon Cour/fiche côte à côte, repli en
colonne, bord droit contraint) — or `map_ui.gd` n'est pas dans la liste de fichiers du lot,
et le brief interdit d'en sortir.

**Choix pris pour ce lot (mécanique, sans trancher la question) :** `CourtPanel` et
`CharacterSheet` ne rejoignent **pas** de zone `UiLayout` — leur parenting/positionnement
dans `map_ui.gd` reste intact (comportement identique). Seuls `UiType` (tailles) et
`UiMotion` (ouverture/fermeture) sont câblés, ce qui ne touche que les fichiers du lot.
**À trancher plus tard** (orchestrateur / PO6b) : soit modifier `map_ui.gd` pour réclamer
ces deux panneaux dans `MODAL` (ou une nouvelle zone compagnon), soit documenter cette
paire comme exception durable à l'ADR 0097 (cas visiblement déjà connu : `province_panel`,
`news_letters`, `EncounterWindow`, `ChronicleWindow` sont réclamés depuis `map_ui.gd`/leur
contrôleur, pas depuis eux-mêmes — mais aucun n'a de compagnon dynamique comme la fiche).

## Autres écarts

- `family_tree_view.gd` et `skill_tree_view.gd` dessinent leur texte en `draw_string` sur un
  `Control` (diagramme, pas des `Label`) avec une échelle `zoom`/`z` déjà propre au composant
  (agrandissement du diagramme). Je ne les fais pas passer par `UiType` (pas un
  `add_theme_font_size_override`, pas un `Label`/`RichTextLabel`/`Button` — hors du champ
  mécanique C3) ; tailles laissées telles quelles.

## État

- [x] Lu `docs/wip/po.md`, ADR 0097, bible DA § 12, plan Phase 2, historique PO1/PO2.
- [x] `.tscn` : tailles de police alignées sur l'échelle à 4 valeurs (`Title`/`Heading`/`Body`/`Caption`).
- [ ] `.gd` : `add_theme_font_size_override` → `UiType.apply`.
- [ ] `UiMotion` sur l'ouverture/fermeture de `CourtPanel` et `CharacterSheet`.
- [ ] C1 (textes d'outil) vérifié sur les deux écrans.
- [ ] `game/tests/p2a_ui_test.gd`
- [ ] `game/tests/p2a_shot.gd` + captures `docs/img/po/p2a/`
- [ ] `smoke.gd` + tests existants de la cour/personnage verts

## Prochaine étape

Terminer la conversion des tailles de police dans les `.gd`, puis câbler `UiMotion`.
