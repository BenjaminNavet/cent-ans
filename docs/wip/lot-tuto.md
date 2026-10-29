# Lot TUTO (boucle juge 2, itération 1)

Branche `worktree-agent-a62be3909ca64d6f4`, base main 99d672914.

## État
1. Tutoriel : 4 étapes ajoutées à `data/tutorial/steps.json` (recruit après build ; supply, siege, battle après governor), conseils par faction (France, Angleterre, Bourgogne) et génériques. Conditions : `recruit` (troupes + recrues en file), `siege` (siège du joueur) dans `game_api::progress::sim_objective_met` ; supply et battle manuelles. Cible `recruit` dans `target.rs`.
2. Réserve de recrutement : infobulle « Hommes » (HUD) + ligne `rec:reserve` du panneau de recrutement. Au passage, l'infobulle disait « Hommes disponibles = réserve » alors que le chiffre est le total des soldats sous les armes : corrigé.
3. Barre du haut : `bar::fit_top_bar` serre l'espacement et réduit les lanceurs sous 1500 px logiques.
4. Prestige : champ supprimé (ADR `XXXX-tuto-prestige-champ-supprime.md`).

## Prochaine étape
Vérifier par dump à 1280x800 (`ui_kit.layout`) que la barre ne déborde plus ; tests ; commit final.
