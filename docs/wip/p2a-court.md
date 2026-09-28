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
- [x] `.gd` : tous les `add_theme_font_size_override` → `UiType.apply` (ou `UiType.size(...)` pour
      les clés secondaires `italics_font_size`/`bold_font_size`, et les appels `IconChip.create`
      de `court_panel.gd`/`character_sheet.gd`).
- [x] `UiMotion.fade_in`/`fade_out` sur l'ouverture/fermeture de `CourtPanel` et `CharacterSheet`.
- [x] C1 (textes d'outil) et C3 (échelle à 4 tailles, 14 px minimum) vérifiés : `p2a_ui_test.gd`
      OK (99 textes lus, tailles vues `[14, 17, 20, 26]`).
- [x] `game/tests/p2a_ui_test.gd` (réutilise les aides de `po_ui_test.gd` par instanciation, sans
      le modifier — son `_init()` attend `process_frame`, jamais émis sur une instance orpheline,
      donc ne se déclenche jamais : sûr, voir commentaire en tête de fichier).
- [x] `game/tests/p2a_shot.gd` + 3 captures `docs/img/po/p2a/` (cour, arbre familial, fiche
      personnage avec arbre de compétences) — réutilise les mises en scène déjà présentes dans
      `campaign_map.gd` (`--stage=court|family_tree|skills`, hors lot, non modifiées).
- [x] `smoke.gd`, `ui1_lettrine_test.gd`, `ui3_test.gd` verts (avant fusion de `main`).
- [x] `git merge main` fait (ff propre, aucun conflit sur les fichiers du lot).

## Blocage à la fusion de `main` (hors lot, à signaler à l'orchestrateur)

Après `git merge main` (qui a ramené entre autres FE0 — titres féodaux, ADR 0098 — et
d'autres lots), `smoke.gd` échoue en masse (`CampaignSim.new_campaign(...) failed`,
`GameDataStore.load(...) : unknown field 'primary_title'` dans `data/factions/fac_aragon.json`
notamment). La bibliothèque `game/bin/libcent_ans.debug.dylib` de `main` (celle que je copie,
consigne CLAUDE.md : « ne pas compiler le core ») date de 01:48, **avant** le commit FE0 (titres,
07:20) qui a changé le schéma des données de faction. Le binaire ne connaît donc pas le nouveau
champ `primary_title` : ce n'est pas une régression de P2a (aucun fichier du lot ne touche à
`core/` ni aux données de faction), mais `main` lui-même semble avoir besoin d'un core reconstruit
après le merge FE0. **Je n'ai pas trouvé de dylib plus récent** dans les emplacements accessibles
à ce worktree. Avant PO6b, quelqu'un doit reconstruire `core` sur `main` et republier
`game/bin/libcent_ans.debug.dylib`.

**Preuve que les fichiers du lot sont sains** : `smoke.gd`, `p2a_ui_test.gd`,
`ui1_lettrine_test.gd`, `ui3_test.gd` étaient tous verts juste avant la fusion (voir journal),
avec exactement le même code que celui livré (seul un `git merge` sans conflit a suivi).

## Prochaine étape

Dès qu'un `libcent_ans.debug.dylib` à jour (post-FE0) est disponible : recopier, réimporter,
relancer `smoke.gd` + `p2a_ui_test.gd` une dernière fois pour confirmer, puis rendre la main à
l'orchestrateur (fusion de la branche : hors périmètre de cet agent).
