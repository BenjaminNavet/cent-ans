# DA5b — Icônes d'entité en miniatures peintes

Branche : `worktree-agent-a19e7d997471a9770` (base main 9bfbf27f, DA5 fusionné).
Bible `docs/design/2026-09-25-bible-da.md` § 8 ; ADR 0065, section « DA5b ». Plafond 6 $.

## État : terminé (en attente de fusion par l'orchestrateur)

- Inventaire : 153 identifiants encore en SVG ; 101 ont une illustration peinte
  (27 unités, 29 bâtiments, 45 techniques) → **101 miniatures dérivées** (gratuites) ;
  **55 générées** : 30 compétences, 10 ressources, 7 régimes, `bld_collegiate_church`,
  7 catégories d'unité (`unit_category_*`, marqueurs et cartes de bataille).
- Catalogue `data/ui/entity_icons.json` + schéma ; générateur `tools/cent_ans_tools/entity_icons.py`,
  CLI `cent-ans assets entity-icons` (`--dry-run`, `--only`, `--limit`, `--build-only`,
  `--envelope`) ; tests `tools/tests/test_entity_icons.py`. Sources brutes `tools/da5b_raw/`.
- Recadrage : automatique (saillance), surcharges `crop` pour 9 images (texte peint, sujet
  décentré). Cadre or bruni + azur peint par code. Sortie `game/assets/icons/entity/` + `index.json`.
- Dépense réelle DA5b : 2,53 $ (sonde 0,14 ; 28 images 1,29, passe interrompue par un délai
  réseau ; 24 images 1,10). Aucune reprise nécessaire.
- Godot : `IconLibrary` lit l'index `entity/` en premier (miniature > encre > SVG, `is_entity`,
  jamais teintée ; `decorate_button` met `expand_icon` pour ne pas gonfler la taille minimale —
  l'arbre des techniques se chevauchait sans cela). Marqueurs de bataille, cartes d'unité et du
  bilan : miniature de catégorie, posée pleine plaque. Smoke : vérifs DA5b dans `_run_icons`.
- Captures `docs/img/da5b/avant_*.png` / `apres_*.png` (ville, techniques, bataille), planche
  `planche_miniatures.png` (32/64/128 px).

## Limites / suites

- Les miniatures dérivées de scènes de foule restent chargées à 32 px et moins (lisibles à 64).
  Une passe de génération « un sujet » pour les plus confuses (tech médicales, ateliers) coûterait
  ≈ 0,05 $ pièce.
- Traits (59) : toujours affichés par catégorie (encre) ; pas de miniature par trait.
- Édits (`edict_section`) et retenue gardent leurs icônes de repli (encre).
- Boutons désactivés : la miniature est grisée par la modulation par défaut du bouton.
