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
- [x] 3. IA Normandie ouest. Diagnostic : le setup classait « frontière » un **port** ou une province
      dont un voisin de `Province::neighbors` (données, souvent vide) avait un autre propriétaire
      → garnison de 3 ; l'IA ne comptait que les voisins **en guerre** (graphe `edges`, terre + mer)
      et ne gardait que 1 + 1 unités ailleurs → elle scindait l'excédent en armée sans général. Pas
      seulement Normandie ouest (voisine de la Bretagne, alliée) : 26 provinces portuaires sur toute
      la carte au tour 1. Correctif : module `sim-campaign/src/frontier.rs`
      (`CampaignState::is_frontier`, `garrison_role`, `GarrisonRole::garrison_size`) = seule source
      de vérité ; frontière = port ou voisine terrestre (`movement::land_neighbors`, géométrie
      d'abord) contrôlée par une autre faction. Setup (tailles de garnison) et IA (garnison gardée
      = taille − 1, sites de recrutement, valeur des fortifications) l'utilisent. Tests
      `core/crates/ai/tests/p1_frontier.rs`.

## Prochaine étape
Terminé — fusion par l'orchestrateur.
