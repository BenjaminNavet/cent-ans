# P1 — petits points cœur

Branche dédiée (worktree), fusion par l'orchestrateur.

## État
- [x] 1. `no_quarter` en campagne : `resolve_pending_battle` (sim-campaign/src/battle_request.rs).
      Vainqueur « pas de quartier » → chef vaincu pris = tué (pas de captif, pas de rançon H6),
      chef vainqueur −5 piété (`NO_QUARTER_PIETY`), ligne de journal. Tests `tests/p1_no_quarter.rs`.
- [x] 2. Étain : déjà utilisé par `bld_tin_blowing_house` (commit b0a5253, historien, avant P1).
      Décision : **pas d'extension du schéma** — `required_resource` reste un identifiant unique.
      Aucun bâtiment n'a besoin de plusieurs ressources ; passer à un tableau toucherait data-model,
      l'UI (encyclopédie, infobulles, mock) pour rien (YAGNI, risque de conflit). Ajouts P1 : tests
      pytest `tools/tests/test_buildings_schema.py` (schéma de tous les bâtiments, ressources requises
      existantes et présentes sur la carte, étain → maison de fonte), test Rust `tests/p1_tin.rs`
      (constructible en Cornouailles/Devon, refusée dans le Kent « ressource requise absente »),
      paragraphe « blowing houses » dans `cdx_tin_stannaries`.
- [ ] 3. IA Normandie ouest (classification frontière).

## Prochaine étape
Point 3.
