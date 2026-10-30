# Lot R1-UI — renommée et ambitions dans le client Bevy (boucle juge 2)

Source : ADR 0805. Client + petite extension de `game_api::acts`.

## État
- [x] Renommage `policies`/`choose_policy`/`ActPolicyRow` -> `ambitions`/`choose_ambition`/`ActAmbitionRow` (game-api + client)
- [x] `ActView` : seuil de victoire, choix ouvert ; en-tête du panneau d'objectifs (seuil)
- [x] Panneau d'objectifs : choix des ambitions (2 parmi 4, bonus de renommée)
- [x] Écran de transition : renommée de maison, 3 dernières sources, seuil, ambitions avec bonus
- [x] Dump d'état (`progress_objectives.view.act`, `progress_acts.view`)
- [x] TUTO : le champ `prestige` de faction a été supprimé (ADR 0796) ; rien à brancher (la renommée est déjà
  l'en-tête du panneau d'objectifs)

## Prochaine étape
Clippy/tests client, smoke, dump d'état via run_bg.sh.

## Vérifié
Clippy client et tests game-api verts ; test client : 1522 verts, 1 échec étranger (`markers::dn::real_table_and_glbs`, glb absent du worktree). Dump `out/r1ui.json` : en-tête avec seuil 43, 4 ambitions à 50 %, écran de transition (renommée 10, 3 sources). Smoke : voir rapport.
