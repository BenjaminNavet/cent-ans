# WIP — C3 personnages à la Total War (arbre familial, fiche de général)

Spéc : lot C3 de `docs/design/2026-09-24-rapprochement-total-war.md` (T5 de `2026-09-24-analyse-total-war.md`).

## État
- [x] Pont : `CampaignSim.get_family_tree(character, up, down) -> {root, ruler, heir, nodes}`
  (`core/crates/godot-bridge/src/campaign_sim_family.rs`, lecture seule, aucune règle).
- [ ] Vue `game/scripts/ui/family_tree_view.gd` (médaillons, traits à l'encre, défilement/zoom).
- [ ] Onglet « Arbre familial » dans `court_panel.gd`.
- [ ] Fiche de personnage à la TW (`character_sheet.gd`) : grand portrait, écu, pastilles de traits,
  arbre de compétences visuel (`skill_tree_view.gd`).
- [ ] Smoke étendu.
- [ ] Captures `docs/img/c3/`.

## Prochaine étape
Écrire `family_tree_view.gd` puis l'onglet du panneau Cour.

## Notes
- Aucune date de décès n'est gardée par l'état (seulement `alive` et `birth_year`) : l'arbre affiche
  « ° année · † » pour un défunt.
- La France n'a que 2 générations vivantes en 1337 ; Charles V naît à l'hiver 1338 (8 fins de tour).
- Pas de concept de suite/entourage dans le cœur : non inventé (piste).
