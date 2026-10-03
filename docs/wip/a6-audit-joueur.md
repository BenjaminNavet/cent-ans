# A6 — Audit joueur : corrections

Tableau des constats : `docs/audit/a6-audit-joueur.md`. Notes par lot : `docs/wip/a6-l*.md`, `a6-integration-murs.md`.

## État (03/10) : FUSIONNÉ dans main
- **Rust :** suite complète verte sur a6-merge (212 binaires de test, clippy propre).
- **main :** pytest 1616 passés ; smoke et tests Godot A6 OK. Le smoke a été corrigé après la fusion : la limite de la bataille sans ordre passe de 720 s à 1 200 s (ADR 0184, addendum).
- **Modifications sur le mouvement libre :** elles étaient non commitées dans le checkout principal (03/10 06:29) et sont sauvegardées sur la branche `wip/free-movement-orphan`.
- **Reste à faire :**
  - vérification visuelle : bataille, barre d'emplacements, choix de faction, carte (`a6_map_shots.gd`) ;
  - mettre à jour l'état par constat dans `docs/audit/a6-audit-joueur.md`.

## Lots
| Lot | Constats | État |
|---|---|---|
| L1 prévision = résolveur réel échantillonné (100 graines) | M1 M2 | fait |
| L2 sièges : murs, coût des engins selon le niveau, poids des murs dans le résolveur | M3 | fait |
| L3 économie à l'échelle : trésors, rançons, banqueroutes, révoltes | M4 M5 | fait |
| L3b plafond de milice (27,6 %), armées de départ dans `data/rules/starting_armies.json` | M4 | fait |
| L4 recherche : réserve, file | M7 U5 | fait |
| L5 diplomatie | M6 U15 U16 | fait |
| L6 fenêtres et notifications | U6 U7 U8 U10 U17 U20 | fait |
| L7 colonie : files, défilement | M8 U11-U14 | fait |
| L8 sélection et barre du haut | M9 U4 U9 U19 U21 | fait |
| L9 lisibilité de la carte | C1-C9 U18 | fait ; jugement visuel à faire |
| L10 performance : ombres, ultra, fuite | P2-P4 | fait ; 6 821 appels de dessin non reproduits |
| L11 menus d'accueil | U1-U3 | fait |
| L12 interface et caméra de bataille | U22 U23 B2-B4 | fait |
| L13 + L13b durée des batailles : médiane ×1,65, mêlée ×1,9 | B1 | fait |
| L14 sol de bataille | B | fait |
| L15 barre d'emplacements de la colonie | U | fait |

Les ADR de l'audit ont été renumérotés de 0181 à 0185, car leurs numéros entraient en collision avec ceux du chantier LR.

## Points ouverts
- **Bataille sans ordre :** elle se termine en environ 17 minutes (horloges de refus et d'accalmie ×1,9) ; voir l'addendum de l'ADR 0184.
- **Durée des batailles :**
  - la bataille de campagne type n'est allongée que de ×1,38 ;
  - les cartes historiques (Crécy, Azincourt, Poitiers) gardent l'ancien rythme ;
  - `approach_range_m` 345 est un réglage sur le fil ;
  - la marche à ×0,45 peut sembler molle : à juger en partie pilote.
- **Révoltes :** `occupation_unrest` 24 (A6) a été retenu face à 12 (LR) ; à remesurer avec la garnison par habitant de LR.
- **France :** solde de départ à +23 % du revenu ; personne n'a encore joué de partie France à la main pour juger l'argent.
- **Lot possible :** encombrement au début de partie (popup de tutoriel et citation du chroniqueur).
