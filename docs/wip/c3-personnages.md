# WIP — C3 personnages à la Total War (arbre familial, fiche de général)

Spéc : lot C3 de `docs/design/2026-09-24-rapprochement-total-war.md` (T5 de `2026-09-24-analyse-total-war.md`).

## État
- [x] Pont : `CampaignSim.get_family_tree(character, up, down) -> {root, ruler, heir, nodes}`
  (`core/crates/godot-bridge/src/campaign_sim_family.rs`, lecture seule, aucune règle). Nœuds :
  `{id, name, epithet, sex, alive, birth_year, age, house, faction, title, generation, blood,
  father, mother, spouse, children}` ; ascendants (`up`), fratrie, descendants de la racine et de
  sa fratrie (`down`), conjoints ; plafond 120 nœuds.
- [x] Vue `game/scripts/ui/family_tree_view.gd` (`FamilyTreeView`) : médaillons de portrait (écu de
  faction pour les personnages générés), traits à l'encre, double trait des conjoints, fratrie
  sans parents connus en pointillé, défunts grisés « ° année · † », couronne héritier/dirigeant,
  clic = fiche, clic droit = recentrer, Ctrl + molette / boutons = zoom, glisser = défilement.
- [x] Onglet « Arbre familial » dans `court_panel.gd` (le panneau s'élargit à 1180 px).
- [x] Fiche à la TW (`character_sheet.gd` + `.tscn`, 1000 px) : grand portrait encadré d'or + écu,
  titres courants, pastilles âge/sexe/piété/prestige, niveaux, famille (noms résolus), actions ;
  à droite description Codex, traits en pastilles colorées par catégorie (infobulles riches),
  arbre de compétences visuel `game/scripts/ui/skill_tree_view.gd` (`SkillTreeView`). Rançon (G1)
  conservée.
- [x] Smoke étendu (`_check_family_tree_c3`) : onglet, nombre de nœuds = pont, ≥ 3 générations,
  héritier présent, clic médaillon, apprentissage d'une compétence par clic dans l'arbre.
- [x] Captures `docs/img/c3/arbre-valois.png`, `docs/img/c3/fiche-edouard-iii.png`
  (`godot --path game --script res://tests/c3_screenshot.gd`).

## Prochaine étape
Lot terminé, en attente de revue/fusion.

## Notes / points ouverts
- Aucune date de décès n'est gardée par l'état (seulement `alive` et `birth_year`) : l'arbre affiche
  « ° année · † ». Ajouter `death_year` à `CharacterState` serait une évolution du cœur (hors lot).
- Les ascendants morts avant 1337 (Charles de Valois, Philippe IV…) n'existent pas dans l'état :
  l'arbre des Valois commence à Philippe VI. Charles V naît à l'hiver 1338 (8 fins de tour).
- Pas de concept de suite/entourage dans le cœur : non inventé (piste : « suite » à la TW,
  compagnons attachés au général avec effets, nécessiterait données + règles dans `core/`).
- Régimes, médecine et ordres de chevalerie ne sont pas des données de personnage (province /
  faction) : rien à reprendre sur la fiche.
- La fiche (1000 px, ancrée à droite) recouvre le bord du panneau Cour sous 1500 px de large.
