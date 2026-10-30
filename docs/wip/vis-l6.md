# VIS-L6 « bataille-matière »

Spec : `docs/audits/2026-10-30-juge-visuel-iter3.md` (### L6, défaut G2-7). Rendu de bataille seulement.

## Critères
- sol de bataille près : s ≥ 0,38 (pré s ≥ 0,40, v ≤ 0,50 au premier plan) ;
- rang de chevaliers : s moyen ≥ 0,25 (surcots/caparaçons aux couleurs de la bannière) ;
- aucune silhouette identique côte à côte (échelle et teinte par instance).

## Fait (code, non mesuré)
- Sol : le « décor atténué » codé en dur (saturation 0,6, `weather.w`) devient `look.saturation` de
  `data/fx/battle_ground.json` (1,12) ; touffes (`tuft_*`, bruit à 3 échelles, fondu avec la distance) dans `ground.wesl`.
  `battle_ground.json` n'était lu par aucun code Rust : seul `look` l'est (les `noise_scales` restent des constantes WGSL).
- Chevaliers : `heraldry.rs::field_colour` extrait la teinte dominante du champ de l'écu porté (couche du seigneur) ;
  `RegimentArms.dye` remplace la couleur de livrée des régiments nobles (surcot) et dessine le caparaçon du cheval
  (`skinned.wesl`, branche C_COAT, bande d'ourlet couleur de garniture). Désaturation de livrée 0,3 → 0,2.
- Variation : taille par instance des cavaliers (avant : 0), `size_jitter_mounted/foot` dans `battle_soldier_look.json`.
- Horizon : deux plans d'arbres (`plane_split_m`, hauteur et clairières du plan proche) dans `far_woods.rs`.

## À faire
- Une capture de contrôle (battle_low) + mesure HSV ; caler `look.saturation`, bande du caparaçon (hauteur d'ourlet devinée).
- Fumée de cheminée en volume : non fait.
