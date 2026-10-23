# 0745 — Ombre de relief et occlusion d'horizon cuites dans le cache du sol

Date : 2026-10-23 · Lot relief-ombre-ao

## Contexte

Les cascades du soleil s'arrêtent à ~650 ; au-delà (et entre elles) le relief ne projette aucune
ombre, et les vallées n'ont pas d'occlusion d'horizon hormis l'AO RV-D grossière. Solari est
abandonné en campagne. La session perf (ADR 0742) impose : pas de calcul par pixel et par image
dans le fragment principal.

## Décision

Ombre du soleil par marche sur la carte d'altitude `height_lod` (blocs de 16 px, `textureLoad`) et
occlusion d'horizon (8 directions, 3 distances), calculées **dans l'étage sommet de la cuisson**
du cache par nœud (un calcul par sommet du nœud, interpolé sur la couche), écrites dans un atlas
`Rg8Unorm` (r soleil, g ambiant). Le fragment principal ne fait qu'une lecture.

- Le rayon est exprimé en mètres : montée par px = tan(élévation) / (échelle verticale x (1 + gain)).
  Approximation : l'écrasement des vallées (`squash`) est ignoré.
- Signature du cache : direction du soleil quantifiée (1 degré), échelle verticale quantifiée,
  réglages, poignée de `height_lod` ; recuisson au plus 1 fois/s (déjà en place).
- Aucun sampler ajouté (2 bindings : uniforme 43, texture 44).
- Désactivé (`enabled:false`, cache du sol coupé, carte d'altitude absente) : uniforme nul, rendu
  identique à avant.

## Conséquences

Coût : une troisième cible de cuisson (Rg8, +50 Mo d'atlas à 256 texels) et ~55 lectures de
texture par sommet de nœud cuit. Résolution de l'ombre limitée par les blocs de 16 px et par
l'espacement des sommets (flou à grande distance, plausible comme pénombre).
