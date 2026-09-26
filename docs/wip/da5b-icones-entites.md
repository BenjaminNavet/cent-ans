# DA5b — Icônes d'entité en miniatures peintes

Branche : `worktree-agent-a19e7d997471a9770` (base main 9bfbf27f, DA5 fusionné).
Bible `docs/design/2026-09-25-bible-da.md` § 8 ; complète l'ADR 0065 (section DA5b). Plafond 6 $.

## État

- [x] Inventaire : 153 identifiants encore en SVG ; 101 ont une illustration peinte
  (27 unités, 29 bâtiments, 45 techniques) ; sans illustration : 30 compétences, 10 ressources,
  7 régimes, `bld_collegiate_church`, catégories d'unité (marqueurs de bataille).
- [x] Catalogue `data/ui/entity_icons.json` + schéma, générateur `tools/cent_ans_tools/entity_icons.py`
  (recadrage automatique, cadre or/azur peint par code), build des 101 dérivées.
- [ ] Revue des recadrages (surcharges `crop`), image de référence, CLI `cent-ans assets entity-icons`, tests.
- [ ] Génération : dry-run, sonde 3, puis 55 miniatures (≈ 2,5 $).
- [ ] Branchement `IconLibrary` (index `entity/` prioritaire), marqueurs de bataille, cartes d'unité.
- [ ] Planche 32/64/128, captures avant/après `docs/img/da5b/`, ADR 0065 § DA5b.

## Prochaine étape

Voir la première case non cochée.
