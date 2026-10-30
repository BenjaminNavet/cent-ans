# Lot R1-UI — renommée et ambitions dans le client Bevy (boucle juge 2)

Source : ADR 0805. Client + petite extension de `game_api::acts`.

## État
- [x] Renommage `policies`/`choose_policy`/`ActPolicyRow` -> `ambitions`/`choose_ambition`/`ActAmbitionRow` (game-api + client)
- [ ] `ActView` : seuil de victoire, choix ouvert ; en-tête du panneau d'objectifs (seuil)
- [ ] Panneau d'objectifs : choix des ambitions (2 parmi 4, bonus de renommée)
- [ ] Écran de transition : renommée de maison, 3 dernières sources, seuil, ambitions avec bonus
- [ ] Dump d'état
- [x] TUTO : le champ `prestige` de faction a été supprimé (ADR 0796) ; rien à brancher (la renommée est déjà
  l'en-tête du panneau d'objectifs)

## Prochaine étape
Étendre `ActView`, puis UI.
