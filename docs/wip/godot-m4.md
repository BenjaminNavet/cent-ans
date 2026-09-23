# WIP — M4 Godot (personnages et dynasties)

État : démarrage. Implémentation contre `campaign_sim_mock.gd` (extension avec l'API §3),
puis UI (court panel, character sheet, arbre de compétences), puis smoke test, puis captures.

## Plan
1. [ ] `campaign_sim_mock.gd` : charge `data/characters/*.json` en `_characters`, arbre de
   compétences (30 nœuds, 3 branches, hardcodé + repli `data/skills/*.json` si présent),
   `get_character`, `get_faction_characters`, `get_skill_tree`, `get_learnable`,
   `get_marriage_candidates`, ordres `learn_skill`, `assign_governor`, `assign_general`,
   `propose_marriage`, `debug_grant_xp`. Naissances (hiver, couples mariés) et morts
   (probabilité liée à l'âge) en fin de tour ; succession simplifiée sur mort du dirigeant.
2. [ ] `scenes/ui/court_panel.tscn` + `court_panel.gd` : liste, tri, filtre par rôle.
3. [ ] `scenes/ui/character_sheet.tscn` + `character_sheet.gd` : compétences/XP/traits/famille,
   arbre de compétences (3 colonnes), boutons gouverneur/commandement/marier.
4. [ ] `map_ui.gd` + `campaign_map.tscn` : bouton « Cour », touche C, branchements de signaux,
   couleurs de journal (birth/marriage/death/succession/regency/trait_acquired).
5. [ ] `province_panel` : ligne « Gouverneur : X » + bouton ; `army_panel` : « Général : X »
   + compétences.
6. [ ] `tests/smoke.gd` : court panel, debug_grant_xp + learn_skill, assign_governor,
   propose_marriage, 40 end_turn → naissance ou mort.
7. [ ] Captures `--stage=court` / `--stage=skills`, relecture avec Read, ajustement layout.
8. [ ] `docs/godot-map.md` à jour, suppression de ce fichier, commit final.

## Notes
- `data/skills/` et `data/traits/` n'existent pas encore au moment d'écrire ceci (agents
  `data/` en parallèle) : repli hardcodé actif tant qu'ils manquent.
- `core/crates/godot-bridge` : pas encore de commits liés à M4 API (`get_character` etc.) au
  démarrage — à revérifier périodiquement (`git log -- core/crates/godot-bridge`).
