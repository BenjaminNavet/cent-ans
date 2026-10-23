# 0744 — Modèles d'arbres proches en lod2 au-delà de 35 unités

Date : 2026-10-23 · Chantier camp60 (60 i/s en carte de campagne)

## Contexte

Autour du point visé, les arbres sont dessinés en modèles 3D (lod1, ≈ 1 360 triangles, rayon
0,6 × distance, borné à 8..40). À d = 60, le rayon atteint 36 et la passe de ces modèles coûte
≈ 11 ms (sans modèles : 18,3 ms contre 29,1). Le lod2 de l'ombre (≈ 290 triangles) dessiné à
leur place ramène d = 60 à 22,6 ms : la passe est limitée par les sommets.

## Décision

À l'assemblage, un modèle proche à plus de `MODEL_LOD1_DISTANCE` (35 unités, ≈ 60 px de haut
pour un arbre à la résolution du joueur) de la caméra va dans un lot à part (clé de rang marquée
`FAR_MODEL_BIT`) qui dessine le lod2 de l'espèce, à défaut le lod1. Réglage d'essai :
`CENT_ANS_MODEL_LOD1_DISTANCE`.

## Conséquences

Réglages du joueur, p50 : d = 60 29,1 → 22,4 ms ; d = 30 21,8 → 21,2 ; d = 150 inchangé.
Captures (vues de référence camp_0025 et camp_0060) avec et sans le palier : écart moyen
0,15 / 255, forme et densité des arbres identiques à l'œil. Un lot de plus par espèce dans la
zone des modèles.
