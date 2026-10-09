# 0233 — Ajustement JR4b des garnisons précalculé

Statut : accepté (10-09, lot SC CC9)

## Contexte
Au démarrage de 1337, `fit_starting_garrisons` (lots JR4b, LR-15, A6-L3) faisait tourner une boucle
d'économie par royaume : plafond de trésorerie, renvoi des garnisons les plus coûteuses, puis des
unités de campagne, de la garnison de capitale et des bâtiments. Le résultat ne dépend que des
données ; le recalculer à chaque partie coûtait du temps et gardait de la logique de réglage dans le
chemin de démarrage.

## Décision
- `data/rules/starting_fit.json` (schéma `starting_fit.schema.json`, `data_model::StartingFit`)
  liste ce que le budget retire : trésoreries plafonnées, positions des unités de garnison, d'armée
  et des bâtiments retirés.
- `new_1337` construit l'état non ajusté (`new_1337_unfitted`) puis applique le fichier
  (`starting_fit::apply_starting_fit`). Sans fichier, aucun ajustement.
- L'algorithme reste, réservé aux tests (`starting_fit::compute`) : un test vérifie que le fichier
  égale le calcul, un autre que l'application redonne exactement l'état calculé. Régénération :
  `CENT_ANS_REGEN_STARTING_FIT=1 cargo test -p sim-campaign starting_fit`.
- Au passage, `agents.rs` utilise `checked_div` (clippy `manual_checked_ops`).

## Conséquences
Partie identique à graine égale. Toute modification des données de départ (armées, garnisons,
bâtiments, économie, `starting_budget`) fait échouer le test tant que le fichier n'est pas
régénéré. Le calcul est fait pour un joueur (France) : le départ ne dépend pas du joueur.
