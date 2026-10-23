# ADR 0760 — Masse, variété et cadrage des armées en bataille

Amende ADR 0016 (taille des unités visuelle), ADR 0174 (formations) et ADR 0404 (garde de performance bataille).

## Contexte

Le juge visuel (out/juge/iter-02-art.md, constats 2, 12, 13, 14, 19) trouve les armées trop
minces : lignes de 3 à 5 px, grands vides, caméra d'ouverture haute dont la moitié basse est de
l'herbe vide. Constat de code : les formations de `data/rules/unit_formations.json` ont déjà 4
rangs (fantassins), 3 (tireurs), 2 (cavaliers) ; la densité dessinée est le réglage client
`unit_size` (figurines par soldat, défaut 1,0) plafonné par `max_figures`. Le moteur rend déjà des
imposteurs au-delà de `battle_lod_impostor_m`, des clips de repos aléatoires (`idle_look`,
`idle_lean`…), des phases et cadences décalées, des couleurs d'étoffe par unité (`unit_looks.json`).

## Décision

1. **Règles inchangées** : rangs, espacements et empreintes de `core/` ne bougent pas (équilibre,
   contacts, collisions). La masse est obtenue côté rendu : `unit_size` par défaut relevé (voir
   ci-dessous), les figurines supplémentaires se serrent dans le même rectangle simulé avec plus
   de rangs (ADR 0016).
2. **Jitter de profondeur** : `Unit::figure_positions` décale les hommes 1,6 fois plus en
   profondeur qu'en largeur (`DEPTH_JITTER`) ; rendu seulement, ni contact ni règle ne le lisent.
3. **Caméra d'ouverture plus basse** : distance d'ouverture 100 m → 75 m et inclinaison
   automatique abaissée de 9° dans la plage proche (plein jusqu'à 110 m, nul à 275 m), plancher
   à l'inclinaison de contact. Le réglage manuel (bouton du milieu) n'est pas touché.
4. **Variété** : la palette d'étoffes délavées/rapiécées par type d'unité reste dans
   `data/fx/unit_looks.json` ; aucun nouvel asset (pas de Meshy).

## Conséquences

- Le fps de bataille est la contrainte dure : mesures relatives `tools/bevy_bench/bench.sh`
  (voir docs/wip/armees-densite.md pour les chiffres).
- Le lot LUM garde `lighting.rs`, `atmosphere.json` et les LUT.
