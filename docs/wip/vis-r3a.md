# VIS-R3a : carte de couleur rebâtie

Spec : `docs/audits/2026-10-29-visuel-campagne.md` (g) Lot R3, sous-lot R3a ; palette (c). Branche de travail : worktree, pas de fusion.

## État : terminé
- [x] palette `colormap_style.yaml` : ordre de valeur, `crop_contrast` 0,85, routes (0,55 0,48 0,36)
- [x] classes régionales (section `regional` du style, `regional_masks`) : bocage, beauce, meseta, landes
- [x] noyaux villageois (`villages`, `_paint_villages`)
- [x] tests (hiérarchie de valeur, classes régionales, village) ; `uv run --project tools pytest tools/tests` : 4075 passés
- [x] régénération `geo colormap`, mesures avant/après
- [x] bible (c) : labour 0,62 0,49 0,35, blé en herbe 0,40 0,53 0,23

## Résultats (luminance moyenne par classe, texels proches des couleurs de classe, L1 < 0,10)
| classe | avant | après |
|---|---|---|
| forêt | 0,275 | 0,272 |
| pré | 0,540 | 0,524 |
| labour | 0,476 | 0,539 |
| chaume | 0,600 | 0,595 |

Avant : labour < pré (ordre violé). Après : forêt < pré < labour < chaume. Moyenne des terres : 0,460 avant et après.

## Livraison
La carte de couleur (`data/map/colormap_bc1_*.bin`, `colormap_preview.jpg`) est suivie par le dépôt (ADR 0142) ; elle n'est pas dans le
paquet de relief (`relief_pack`, `relief_update`, `launch.sh` n'y touchent pas) : aucune release à publier. Livraison = commit des
fichiers régénérés (`uv run --project tools cent-ans geo colormap`, quelques minutes). Ancienne carte :
`~/dev/cent-ans-raw/colormap-before-vis-r3a/`.

## Points ouverts
- Routes à 0,55 0,48 0,36 (spec) : sous le chaume en valeur, quasi invisibles dans la carte ; le trait vectoriel du client doit porter.
- Aucun contrôle client (pas de capture) : la vue campagne n'a pas été rendue.
