# TS-T2b — disques noirs sous les arbres, fond de vallée noir, taches sur la falaise

Branche `ts/t2b`, worktree `/Users/jean_hubert/dev/gp-ts-t2b`. Suite de `docs/wip/ts-t2.md`.
Vue de contrôle : caméra 5.9,45.25,55 (été / hiver).

## État (terminé)
- Disques noirs : socle plat au pied de modèles générés (épicéa : 11 % des triangles, disque large
  comme le houppier). Sur pente, il dépasse en aval, normales radiales vers le bas → noir.
  Correctif : `render.rs::foot_plate_free` retire les triangles bas (< 8 % de la hauteur) hors du tronc.
- Fond de vallée noir, parts mesurées (luminance sRGB de la vallée, captures `CENT_ANS_PERF_OFF`) :
  post ≈ moitié de l'écrasement (contraste linéaire autour de 0,5 puis courbe en S), ombres du
  relief ≈ 40 %, canopée et sol forestier ≈ 0. Correctif données : contraste des saisons 1,02
  (`data/fx/atmosphere.json`), `value_contrast` 0,16 (`data/fx/campaign_post.json`).
  Vallée 0,10 → 0,17 (été), 0,25 (hiver). La brume des creux T3b à d = 55 ne change presque rien
  (0,169 → 0,167) : pas de réglage.
- Taches brun-noir sur la falaise : sous-bois T7 (lisière `belt_*`, touffes). Sans sous-bois, la
  falaise est nette ; les rochers T9 n'y font pas de taches (seulement leur texture granuleuse).
  La pente du sous-bois est la pente réelle (m/m), bien plus faible que la pente affichée (relief
  exagéré). Correctif données : `data/fx/understory.json` `slope` 0,25 / 0,12 → 0,12 / 0,07
  (taches de falaise −72 %, sous-bois des pentes douces gardé à ≈ 80 %).

## Points ouverts
- Résidu de sous-bois sur les crêtes rocheuses (pente réelle faible) : il faudrait tester la roche
  comme le terrain (`rock_w` du splat, ligne de roche en altitude) dans `understory.rs` (lot T7).
