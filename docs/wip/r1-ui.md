# Lot R1-UI — renommée et ambitions dans le client Bevy (boucle juge 2)

Source : ADR 0805. Client + petite extension de `game_api::acts`.

## État
- [x] Renommage `policies`/`choose_policy`/`ActPolicyRow` -> `ambitions`/`choose_ambition`/`ActAmbitionRow` (game-api + client)
- [x] `ActView` : seuil de victoire, choix ouvert ; en-tête du panneau d'objectifs (seuil)
- [x] (code écrit, à vérifier) Panneau d'objectifs : choix des ambitions (2 parmi 4, bonus de renommée)
- [x] (code écrit, à vérifier) Écran de transition : renommée de maison, 3 dernières sources, seuil, ambitions avec bonus
- [x] Dump d'état (`progress_objectives.view.act`, `progress_acts.view`)
- [x] TUTO : le champ `prestige` de faction a été supprimé (ADR 0796) ; rien à brancher (la renommée est déjà
  l'en-tête du panneau d'objectifs)

## Prochaine étape
Clippy/tests client, smoke, dump d'état via run_bg.sh.
