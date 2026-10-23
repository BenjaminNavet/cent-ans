# 0743 — Ombres d'arbres découpées par cascade

Date : 2026-10-23 · Chantier camp60 (60 i/s en carte de campagne)

## Contexte

Les lots d'arbres de la carte (`VegBatch`) sont regroupés par espèce, pas par zone : chaque lot
couvre toute la vue. Ils étaient dessinés en entier dans chacune des 4 cascades d'ombre du soleil.
Mesures (d = 30, réglages du joueur, cache du sol actif) : sans ombres d'arbres 18,3 ms contre
28,0 ms. La taille de la carte d'ombre n'y change rien (4096 : 28,3 ms) : le coût vient du nombre
d'instances traitées, pas du remplissage.

## Décision

- L'assemblage (`render::assemble`, fil de travail) trie les enregistrements de chaque lot par
  distance horizontale au point au sol de la caméra (`billboard::sort_records`) et garde l'ordre
  (`VegBatch.shadow_order` : distances triées, hauteur maximale, base la plus basse, origine).
- À chaque image (`shadows::shadow_ranges`, `PostUpdate`), chaque cascade reçoit la plage
  contiguë d'arbres qui peuvent l'atteindre (`cascade_range`). Un point à la profondeur de vue z est à
  au plus z / cos θ de la caméra (θ : demi-diagonale du champ). On élargit la plage de la portée
  d'ombre (1,5 × hauteur max / tan de l'élévation du soleil, élévation ≥ 0,08) et du déplacement
  de la caméra depuis le tri.
- `VegShadowCaster` porte les plages ; la file saute une cascade vide ; `DrawShadowBatch` lit
  l'indice de cascade de la vue (`LightEntity`) et dessine `first..last` en instances.
- L'assemblage est relancé quand le point au sol de la caméra a bougé de plus de
  max(4, 0,3 × distance) depuis le tri (rotation sans panoramique).
- Interrupteur de banc : `CENT_ANS_PERF_OFF=vegcull` (toutes les cascades dessinent tout).

## Conséquences

d = 30 : 27,5 → 21,8 ms (p95 30,5 → 24,6), sans ombre retirée. Le tri ajoute un tri par lot sur
le fil d'assemblage. Le tri du plus proche au plus loin sert aussi la passe principale (rejet de
profondeur précoce).
