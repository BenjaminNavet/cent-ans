# WIP — H2 Codex et bulles imbriquées

Conception : `docs/design/2026-09-23-histoire-et-savoir.md` §1. Branche : worktree agent H2.

## État : vague 1 terminée
- [x] Schéma `data/schemas/codex.schema.json` + `codex_id` dans `common.schema.json`
- [x] 20 fiches d'amorce `data/codex/cdx_*.json` + `data/codex/_todo.md` (21 fiches de la vague 2)
- [x] Validateur `tools/cent_ans_tools/codex.py` + `tools/tests/test_codex.py` (schéma, id = fichier,
      liens `[[…]]` dans tout `data/**`, `see_also`, alias uniques, `entity` existante ; liens vers
      `_todo.md` tolérés)
- [x] Autoloads `CodexStore` (fiches, alias, découvertes `user://codex.json`) et `CodexBubbles`
      (pile de bulles, fenêtre, touches K / T / Échap) ; `CodexText` ; `CodexWindow`
- [x] Épinglage des infobulles F2 : `RichTooltip.make_panel` passe le texte par
      `CodexText.format(…, true)` et mémorise `last_panel` / `last_bbcode` ; touche T →
      `CodexBubbles.pin_native_tooltip()`
- [x] Hooks : chronique (auto-liens), fiche personnage (description + lien vers la fiche),
      infobulles des techs (via `make_panel`)
- [x] Smoke `_run_codex` ; captures `docs/img/codex-bubbles.png`, `docs/img/codex-window.png`
      (`godot --path game --script res://tests/codex_screenshot.gd`)

## Brancher une UI
```gdscript
label.bbcode_enabled = true
label.text = CodexText.format(texte, auto_link)   # auto_link = true pour les textes de la sim
get_node("/root/CodexBubbles").attach(label)      # ou CodexBubbles.attach(label) hors smoke
```
Ouvrir une fiche : `CodexBubbles.open_entry("cdx_crecy")` ; fiche d'une entité :
`CodexStore.entry_for_entity("chr_charles_v")`.

## Reste à faire (autres sessions / vague 2)
- Bouton « Codex » du bandeau supérieur : dans `map_ui.gd` (non modifié ici, possédé par une autre
  session) → `get_node("/root/CodexBubbles").call("toggle_window")`, icône à ajouter à `icons.json`.
- Brancher `attach` sur les descriptions du panneau de province et des bâtiments (`province_panel.gd`).
- Fiches de la vague 2 (`data/codex/_todo.md`), puis F8 : fiches générées (unités, bâtiments, techs).
- Onglets « Médecine et herbier » et « Savoirs » masqués derrière les flèches du TabBar à 980 px.
- La touche T sert aussi à l'arbre des technologies : elle n'épingle que si une infobulle riche
  est visible (consommée alors), sinon l'arbre s'ouvre comme avant.

## Prochaine étape
Fusion dans `main` par l'orchestrateur.
