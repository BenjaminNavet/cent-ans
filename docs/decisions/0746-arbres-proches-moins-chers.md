# 0746 — Arbres proches moins chers : zone de modèles réduite et lod2 dès 15 unités

Statut : accepté (23/10)

## Contexte
À distance de caméra 30, la carte de campagne tourne à ≈ 21,7 ms (Ultra, 3456×2234, M4 Pro sur secteur).
Bissection (bascules `CENT_ANS_PERF_OFF`) :
- ombres des arbres ≈ 3,6 ms ; ombres des maillages hors arbres ≈ 1,4 ms (villes, `StandardMaterial`,
  marqueurs et vie : aucun gain mesurable) ; ombre du relief ≈ 0,9 ms ;
- modèles 3D d'arbres proches (vs imposteurs seuls, `CENT_ANS_VEG_MODELS=0`) ≈ 3,3 ms, dont ≈ 2,9 ms
  en passe principale : c'est eux, pas les imposteurs, qui coûtent dans « l'affichage principal des arbres ».

Essais rejetés :
- pré-passe de profondeur des imposteurs puis passe couleur en `Equal` sans `discard` (rendre au GPU TBDR
  l'élimination des pixels cachés) : 22,0 contre 21,7 ms à d30, 23,35 contre 22,9 ms à d60 — retirée ;
- filtre d'ombre `Hardware2x2` ou `Temporal` à la place de `Gaussian` : pas de gain net.

## Décision
- `MODEL_LOD1_DISTANCE` 35 → 15 : 20,1 ms à d30 (−1,6) ; 0 donne le même chiffre.
- `MODEL_RADIUS_FACTOR` 0,6 → 0,4 : 19,8 ms à d30 (−1,9) ; 0,3 donnait 19,2, écarté pour limiter le changement visuel non vérifié.
- Réglages de banc conservés : `CENT_ANS_MODEL_LOD1_DISTANCE`, `CENT_ANS_MODEL_RADIUS_FACTOR`, bascules
  d'ombre par famille (`meshshadow`, `townshadow`, `stdshadow`, `markershadow`, `lifeshadow`, `armyshadow`).

## Conséquences
- La combinaison des deux réglages n'a pas été mesurée (banc interrompu : machine passée sur batterie) ;
  gain attendu ≈ 2-3 ms à d30, à confirmer sur secteur.
- Rendu non vérifié en capture : sous d≈20, couronnes lod2 plus clairsemées et transition
  modèle/imposteur plus proche ; revenir aux anciennes valeurs par les variables ci-dessus si c'est visible.
- `armyshadow` non mesuré proprement ; les ombres des imposteurs (≈ 3,7 ms) restent le plus gros poste.
