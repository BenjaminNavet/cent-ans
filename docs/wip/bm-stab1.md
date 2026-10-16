# Lot stab1 — stabilité du client Bevy

Branche `bm/stab1` (base 60b30bc6e). Six points, par priorité.

## État
1. Arrêt silencieux ~3 s après le début d'une bataille : en cours (boucle de 12 batailles Crécy / siège).
2. Blocage à la sortie : en cours.
3. « Mesh index 17 contains no vertices » : sonde en place.
4. Lueur des feux de camp : `battle/effects/campfires.rs` (port de `attach_smoke` / `_place_campfires`).
5. `params.fx.x` partagé : paramètre propre `hitn.y` (role blend) — fait.
6. Clics `cb:season` / `cb:weather` perdus : piste = `custom_battle_last.json` partagé entre exécutions.

## Prochaine étape
Finir la boucle, corriger 1-3, preuves.
