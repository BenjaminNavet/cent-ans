# IB2 — migration mécanique (tables → données, ~120 infobulles brutes) — fichier de reprise

Branche `feat/ib2-migrate` depuis `main` (05e711e1). Spec `docs/superpowers/specs/2026-09-28-ib-infobulles-design.md`
§ 2.4, ADR 0109, orchestration `docs/wip/ib.md`. IB1/IB3/IB4 déjà dans `main`.

## Plan

1. Tables de `game/scripts/ui/rich_tooltip.gd` (`HUD_TEXTS`, `GAUGE_TEXTS`, `EFFECT_LABELS`,
   `STAT_LABELS`, + catégories/capacités/classes/branches) → `data/ui/tooltips.json`
   (fusion avec les `body` déjà posés par IB4), schéma étendu si besoin, `additionalProperties`
   conservé false. `RichTooltip` lit ces données avec repli (calque sur `CameraFeel`/`texts()`
   déjà existant côté IB4).
2. ~120 `tooltip_text = "…"` littéraux → `RichTooltip.plain(title, body, hint)` (spec
   `kind: "plain"`) ; textes dans `data/ui/tooltips.json` bloc `plain`.
3. `unit_card.gd` : déjà migré par IB1, vérification seulement.
4. Tests : `ib_plain_test.gd` (aucun `tooltip_text` littéral hors exceptions, titre présent),
   pytest `test_tooltip_schemas.py` (clés `plain` référencées existent).

## État

- [x] Squelette (ce fichier, branche, dylib copiée, import fait)
- [x] 1. Tables → `tooltips.json` + schéma (`hud`, `categories`, `abilities`, `classes`,
  `branches` ajoutés ; `effects`/`stats`/`gauges` déjà remplis par IB4, fusionnés sans perte).
  `RichTooltip` lit ces données via `_label`/`stat_label`/`unit_category_label`/…/`branch_text`
  (repli comme `CameraFeel` : `texts()` retombe sur `data/` racine si le dossier de `MapPaths`
  n'a pas le fichier — bogue latent d'IB4 corrigé au passage, voir Journal).
- [x] 2. `RichTooltip.plain_spec`/`plain`/`attach_plain` + `plain_tooltip_host.gd` (script
  générique pour les contrôles natifs sans classe dédiée) ; `spec_for` route `"plain"`.
- [x] 3. Migration des ~131 littéraux `tooltip_text = "…"` faite en 4 lots (menus, carte, ui/
  restant, bataille+naval+codex+audio) ; `LITERAL_EXCEPTIONS` vide, `ib_plain_test.gd` vert
  (grep sans littéral restant hors commentaires).
- [x] 4. Tests : `ib_plain_test.gd` (mécanisme d'attache + grep sans littéraux hors exceptions),
  pytest `test_tooltip_schemas.py` (`plain` référencées existent, `effects`/`stats`/`gauges`
  ont toutes un corps).

## Choix faits pendant le travail

- Mécanisme d'attache : `RichTooltip.attach_plain(control, key, live={})`. Pose la clé
  `ib:plain:<key>` (+ repli BBCode) via `set_tooltip` (déjà générique, IB1) ; si `control` n'a pas
  de script, lui attache `plain_tooltip_host.gd` (`_make_custom_tooltip` → `panel_for`, même
  patron que `RichButton`/`IconChip`/`RichPanel`). Un contrôle déjà scripté (`RichButton`…) garde
  son script, seule la clé change. `live` porte `title`/`body`/`hint` dynamiques, prioritaires sur
  `tooltips.json`.
- Titre/corps : heuristique manuelle par occurrence (le plus souvent un clause avant « : »
  devient le titre, le reste le corps ; sinon un titre court composé pour le contexte). Clés
  génériques réutilisées quand le texte se répète telles quelles (`close_escape`, `close`,
  `open_character_sheet`, `seat_unavailable`…).
- Une bulle `plain` place son texte (`body`/`hint`) en ligne d'effet neutre (`sign: 0`), pas dans
  `detail` : `TooltipView.blocks_for` ne montre `detail` qu'en version verrouillée (Alt), donc un
  simple survol resterait vide sinon.
- `RichLabel` (`set_script(RichLabel)`, ex. `pre_battle_dialog.gd`) route encore vers
  `RichTooltip.make_panel` (BBCode à plat), pas vers `TooltipView` : `attach_plain` y fonctionne
  (clé + repli lisibles) mais sans les sections ; mise à niveau de `RichLabel` hors périmètre IB2
  (même famille que `RichButton`/`IconChip`/`RichPanel`, mais pas listée dans le brief).
- Bogue latent d'IB4 trouvé et corrigé : `RichTooltip.texts()` n'avait pas le repli vers `data/`
  racine que `CameraFeel`/`TooltipView.style()` ont déjà (voir Journal).
- Aucune exception : les ~131 occurrences repérées par le grep du brief sont toutes migrées
  (`LITERAL_EXCEPTIONS` vide). Cas `tooltip_text = ""` (effacement) exclus du contrôle : ce n'est
  pas un texte à migrer.

## Journal

- 28/09 : lecture spec/ADR/wip IB, `rich_tooltip.gd` (1183 lignes), `tooltips.json`, schéma,
  `camera_feel.gd` (patron de repli). Branche créée depuis `main` (05e711e1), dylib copiée depuis
  `../../game/bin`, import headless OK. 131 `tooltip_text = "..."` littéraux recensés par grep.
  Prochaine étape : lot 1 (tables → données).
- 28/09 : lot 1 fait. Bogue trouvé en route : `RichTooltip.texts()` (IB4) n'avait pas le repli
  vers `data/` racine que `CameraFeel`/`TooltipView.style()` ont déjà ; dans `smoke.gd`, `MapPaths`
  démarre pointé sur `game/tests/fixtures` (sans `ui/tooltips.json`), donc toute lecture y échouait
  et se mettait en cache vide pour le reste du process — `effect_text` (plague_resistance…) et
  `RichTooltip.hud()` (posture du sceau) en dépendaient déjà via IB4/CV3-4. Corrigé (repli identique
  à `CameraFeel`) ; `smoke.gd`, `ib_layout_test.gd`, `ib_chain_test.gd` verts. Deux erreurs de
  script trouvées puis corrigées en cours de route (recherche-remplace un peu trop large) :
  récursion infinie dans `effect_label`, `TECH_BRANCH_LABELS.get(...)` mordu par le remplacement
  de `BRANCH_LABELS.get(branch, branch)` — les deux visibles immédiatement au premier `smoke.gd`.
  Lot 2 (mécanisme `plain`) fait : `RichTooltip.plain_spec/plain/attach_plain`,
  `plain_tooltip_host.gd`, `ib_plain_test.gd` (attache sur `RichButton` et sur un `Button` natif,
  `live` dynamique, grep sans littéral hors `LITERAL_EXCEPTIONS`). Choix : corps/raccourci d'une
  bulle `plain` rendus comme des lignes d'effet neutres (`sign: 0`), pas dans `detail`, pour
  rester visibles en survol court (`detail` n'apparaît qu'en bulle verrouillée dans
  `TooltipView.blocks_for`).
- 28/09 : lots 3 (menus, carte, ui/ restant, bataille+naval+codex+audio) faits : ~131 occurrences
  migrées vers `RichTooltip.attach_plain` + `data/ui/tooltips.json` bloc `plain` (une centaine de
  clés). `LITERAL_EXCEPTIONS` vide. Tests verts : pytest, `ib_plain_test.gd` (11 vérifications),
  `ib_layout_test.gd` (253), `ib_chain_test.gd`, `smoke.gd` (voir rapport final pour le dernier
  passage). Reste à l'orchestrateur : fusionner `main`, jugement joueur sur quelques bulles
  `plain` représentatives (le titre/corps est un découpage manuel, pas un texte validé par relecture).
