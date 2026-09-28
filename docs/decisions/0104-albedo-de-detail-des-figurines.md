# 0104 — Albédo de détail généré des figurines fines (lot GA1)

Date : 2026-09-28. Statut : accepté.

## Contexte

Les figurines fines (ADR 0088, FG3) portent huit tuiles répétables 512² calculées en numpy
(RG normale, B relief, A rugosité). De près, les étoffes, cuirs et mailles restent des
motifs synthétiques réguliers ; la couleur vient seulement des sommets (livrée, armoiries
pilotées par les données, ADR 0014). La spec GA (`docs/superpowers/specs/2026-09-28-ga-assets-generes-design.md`)
propose des matières générées par IA, sans recuire les atlas et sans toucher aux couleurs
des données. Contraintes : ajout mémoire ≤ 4 Mo, coût ≤ +5 % au banc, budget ≤ 4 $.

## Décision

1. **Douze matières générées** (`data/art/materials.yaml`, schéma
   `data/schemas/materials.schema.json`) : laine, lin, futaine, gambison, mailles, cuir,
   plates, bois, peau, cheveux, robe de cheval claire, robe foncée. Image 1024²
   `openai/gpt-5-image-mini` (prompt : texture plate vue de face, éclairage diffus uniforme,
   sans ombre ni perspective), rendue tuilable (décalage de moitié + fondu de la croix de
   coutures avec des copies décalées sur un seul axe), réduite à 512² par mosaïque 3×3
   (bords toujours raccordés), cartes dérivées localement : hauteur = luminance passe-haut
   (flou gaussien en `wrap`), normale OpenGL (Y+), rugosité (plus rugueuse dans les creux et
   le sombre, `roughness_bias` par matière). Chaîne : `tools/cent_ans_tools/material_gen.py`,
   CLI `cent-ans assets materials`.
2. **Deux tableaux, même format que FG3.** `fine_detail_ga1.png` (12 couches 512² RGBA :
   RG normale, B relief, A rugosité **absolue**) remplace `fine_detail.png` au chargement ;
   `fine_detail_albedo.png` (12 couches 256² RGB) ajoute un **albédo de détail** dont la
   luminance moyenne vaut 0,5 et dont la teinte est gardée à `detail_saturation` (0 à 0,3).
   Le shader le multiplie (×2) à la couleur de sommet, dosé par matière (`ga_mix`) : en
   moyenne la couleur des données est conservée, livrée et armoiries restent lisibles.
3. **Branchement.** Sous `#ifdef FG3_BAKED` seulement, uniforme `ga1_detail` (flux
   uniforme). Même projection que FG3 (position de repos, `textureGrad`), taille des tuiles
   `GA1_TILE_SIZE` = `tile_m` du YAML (test pytest de cohérence). Étoffe : laine ou lin selon
   le soldat, futaine pour livrée et armoiries ; mailles : albédo généré (anneaux et creux),
   rugosité de la tuile (anneaux ≈ 0,35) ; robes de cheval : claire pour gris et isabelle.
   Aucune recuisson d'atlas.
4. **A/B.** `--no-ga1` après `--` : tuiles FG3 d'origine ; l'ancien tableau n'est alors
   chargé qu'à la place du nouveau.

## Conséquences

- Mémoire (test `game/tests/ga1_maps_test.gd`) : voir « Mesures ».
- Coût réel de génération : voir « Mesures » et `docs/budget.md` (section GA).
- Régénérer une matière : supprimer son `<id>_raw.png` du dossier de sortie, relancer
  `cent-ans assets materials --out <dossier> --only <id>`, puis
  `material_gen.build_fine_arrays(<dossier>)` ; l'ordre du YAML fixe les couches (ajouter en
  fin, reporter `GA1_TILE_*` et `GA1_TILE_SIZE` dans le shader).
- Images brutes hors dépôt ; les deux tableaux sont versionnés (assets « générés, projet »,
  `SOURCE.md` du dossier).
- Visages peints hors périmètre (spec).

## Mesures

- **Mémoire** (`ga1_maps_test.gd`, BC7 + mipmaps) : `fine_detail_ga1` 4,00 Mo (12 × 512²),
  `fine_detail_albedo` 1,00 Mo (12 × 256²), moins l'ancien `fine_detail` 2,67 Mo non chargé :
  **ajout 2,33 Mo** (plafond 4). Cartes des figurines fines : 17,00 Mo (budget 60 Mo).
  L'albédo en 256² (et non 512²) tient le plafond : l'ensemble en 512² ajoutait 5,3 Mo.
- **Coût** : 12 images + sonde à 0,04 $ (estimation OpenRouter 0,02 $), un appel coupé
  facturé ; total GA1 **0,59 $** (plafond 4 $).
- **Raccords** : sur les 12 tuiles 512², écart moyen entre bords opposés ≈ pas moyen entre
  pixels voisins (sans couture). Mailles : rugosité anneaux 0,38, creux 0,52.
- **Banc** `--units=50 --benchmark --bench-at=90 --quality=ultra`, 1600×900, ~11 950
  soldats, passes alternées GA1 / `--no-ga1` (image médiane, ms) :

  | passe | standard GA1 | standard `--no-ga1` | rapproché GA1 | rapproché `--no-ga1` |
  |---|---|---|---|---|
  | 1 | 31,5 | 31,3 | 33,3 | 34,9 |
  | 2 | 46,7 | 43,3 | 51,4 | 52,4 |
  | 3 | 33,3 | 33,3 | — | — |

  Machine partagée avec d'autres sessions (bruit ±20 % d'une passe à l'autre ; une 4ᵉ passe
  aberrante, 12,8 ms en GA1, écartée). Médiane des passes : standard 33,3 contre 33,3 ms,
  rapproché −3 % : **écart dans le bruit, ≤ +5 %** (une lecture de texture de plus, sous
  `fine_distance` seulement).
