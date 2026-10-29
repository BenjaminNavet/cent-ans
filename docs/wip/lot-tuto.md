# Lot TUTO (boucle juge 2, itération 1)

Branche `worktree-agent-a62be3909ca64d6f4`, base main 99d672914.

## État
1. Tutoriel : 4 étapes ajoutées à `data/tutorial/steps.json` (recruit après build ; supply, siege, battle après governor), conseils par faction (France, Angleterre, Bourgogne) et génériques. Conditions : `recruit` (troupes + recrues en file), `siege` (siège du joueur) dans `game_api::progress::sim_objective_met` ; supply et battle manuelles. Cible `recruit` dans `target.rs`.
2. Réserve de recrutement : infobulle « Hommes » (HUD) + ligne `rec:reserve` du panneau de recrutement. Au passage, l'infobulle disait « Hommes disponibles = réserve » alors que le chiffre est le total des soldats sous les armes : corrigé.
3. Barre du haut : `bar::fit_top_bar` serre l'espacement et réduit les lanceurs sous 1500 px logiques.
4. Prestige : champ supprimé (ADR `0796-tuto-prestige-champ-supprime.md`).

5. Panneau de recrutement : zone `SideTall` (hauteur définie) + plafond 100 % : la grille de cartes rétrécit et défile, « Sceller la levée » reste visible à 1280x800.
6. Étape recherche : touche lue du keymap (`{key:map_toggle_tech}`) ; la carte du guide s'écarte (racine retirée, flèche conservée) quand recrutement ou diplomatie est ouvert (`Tutorial::aside`).

## Vérifications
Dump 1280x800 : `hud.bar` width 1293, menu_right 1281, overflows false. Tests core/client verts ; seul échec : `markers::dn::tests::real_table_and_glbs` (GLB absent du worktree, sans lien). pytest 4068 verts.
