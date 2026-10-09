# TX — Textures régionales générées en local (conception)

Date : 2026-10-09. Statut : approuvé par le joueur en séance de conception ; **amendé le même jour** :
le joueur valide fal.ai (enveloppe 10 $) pour aller plus vite, voir « Amendement fal ».

## But

Le jeu manque de textures propres à chaque coin de la carte (désert, steppe, Europe centrale,
Nord…) pour le sol, la végétation et les bâtiments, et les textures existantes manquent de
résolution et de détail de près. Le chantier remplace les textures Poly Haven et les cartes fal
par des textures **générées en local** avec Z-Image Turbo (mflux, ADR 0190), **par biome** pour
les sols et la végétation et **par région** pour les bâtiments, en **2k + micro-détail**.

Périmètre : familles A (sol de campagne), B (sol et végétation de bataille), C (matières de
bâtiments), D (cartes de végétation, feuillages, écorces ; eau, voir § 3). Hors périmètre :
textures cuites dans les modèles 3D DN (maquettes de villes, monuments, accessoires).

## Amendement fal (2026-10-09)

- Générateur par défaut : **Z-Image Turbo sur fal.ai** (`fal-ai/z-image/turbo`, 0,005 $/Mpx), en **2048
  natif** (essai : vrai détail au pixel, pas d'agrandissement). Appels en parallèle (8), quelques
  secondes par image au lieu de ~1 min + 2 min d'agrandissement en local.
- Catalogue : champ `backend: fal | local` (défaut `fal`), taille 1024, 1536 ou 2048.
- Local (mflux + Real-ESRGAN) : route de secours ; un échec fal n'est jamais refait en local dans le
  même lot (listé pour une session locale à part).
- Coût : ~330 images × 0,021 $ ≈ 7 $, enveloppe 10 $ consignée dans `docs/budget.md` ; la commande
  refuse un lot au-delà de `--max-cost`.
- Le reste (raccord, PBR, alpha rembg, micro-détail, paquets) reste local. Les lignes « 0 $ » et
  « local » ci-dessous sont remplacées par cet amendement.

## Décisions prises

| Sujet | Décision |
|---|---|
| Générateur | Z-Image Turbo local (mflux, `~/models/mflux/z-image-turbo-q8`), 0 $, aucun repli payant |
| Résolution | génération 1024–1536, agrandissement local Real-ESRGAN en 2k, texture de micro-détail par famille fondue de près ; pas de 4k |
| Biomes | 14 biomes (indices 1-7 conservés, 8-14 ajoutés), voir § 2 |
| Sélection | une graine par matière, contrôles automatiques, relecture par planches ; refaire seulement les ratés flagrants |
| Outil | généralisation de `tools/cent_ans_tools/ground_materials.py` en fabrique de textures (approche A) ; tuilage natif dans mflux et synthèse de texture écartés |
| Ordre | 1 fabrique → 2a biomes → 2b sols campagne → 2c sols bataille → 3 végétation → 4 bâtiments |

## 1. Fabrique de textures

Paquet `tools/cent_ans_tools/texture_factory/`, unités indépendantes et testables :

| Module | Rôle |
|---|---|
| `catalog` | lit et valide les catalogues de `data/art/textures/` |
| `generate` | Z-Image local, graine fixe par entrée, cache brut hors dépôt `~/dev/cent-ans-raw/textures/<famille>/` |
| `seamless` | raccord par découpe à erreur minimale (code actuel de `ground_materials.py`, repris tel quel) |
| `upscale` | Real-ESRGAN local (licence BSD), 1024 → 2048 |
| `pbr` | hauteur estimée (luminance passe-haut) → normale OpenGL + rugosité ; carte de micro-détail |
| `alpha` | détourage rembg local (cartes de végétation, planches de feuilles) |
| `checks` | erreur de bord après raccord, luminance et saturation dans la plage de l'entrée, absence de texte/cadre ; pour les cartes : remplissage, centrage, bord non touché, solidité (scores de `dn_cards_manifest`) |
| `pack` | tableaux de textures (albédo, normale+rugosité) + manifeste |
| `board` | planche de relecture, chaque texture tuilée 2 × 2, hors dépôt |

Commande : `cent-ans textures <famille> generate|seamless|upscale|pbr|pack|board [--only ids]`.
Chaque étape lit la sortie de la précédente sur disque : un lot interrompu reprend où il s'est
arrêté.

Catalogues (un par famille, chacun validé par un schéma de `data/schemas/`) :
`data/art/textures/ground_campaign.yaml`, `ground_battle.yaml`, `vegetation_cards.yaml`,
`foliage_bark.yaml`, `building_materials.yaml`. Champs d'une entrée : `id`, `biomes` (ou
`regions`), `role`, `prompt`, `tile_m` (taille réelle de la tuile), `seed`, `checks` (plages).
Les prompts vivent dans les données, jamais dans le code.

Micro-détail : une texture de grain 2k par famille (sol, écorce, maçonnerie), tuilée finement,
mélangée par les shaders en fondu selon la distance caméra.

Les textures Poly Haven actuelles restent derrière un drapeau `--legacy-textures` jusqu'à la
partie pilote du joueur.

ADR : un pour la fabrique et le passage au local, un pour les 14 biomes.

## 2. Sols

### 2a. Biomes

`data/map/biomes.yaml` gagne les classes 8-14 et leurs règles (Köppen, latitude, boîtes lon/lat,
distance à la mer) ; `biomes.png` est recuit par `cent-ans geo biomes`. Chaque classe déclare un
`parent` (repli pour tout consommateur qui ne la connaît pas).

| # | Biome | Exemples | Parent |
|---|---|---|---|
| 1 | Océanique nord (bocage) | Bretagne, Angleterre, Irlande | — |
| 2 | Continental ouest (openfield) | Bassin parisien, Rhénanie, Bohême | — |
| 3 | Méditerranéen humide | Provence, Toscane, Catalogne | — |
| 4 | Steppe | pontique, kazakhe | — |
| 5 | Taïga boréale | Russie du Nord, Finlande | — |
| 6 | Montagnard / alpin | Alpes, Pyrénées, Caucase | — |
| 7 | Semi-aride | Meseta sèche, Anatolie, hauts plateaux du Maghreb | — |
| 8 | Désert | Sahara, Arabie, Sinaï | 7 |
| 9 | Toundra / arctique | Islande, Laponie | 5 |
| 10 | Continental est / forêt-steppe | Ukraine du Nord, Moscovie | 2 |
| 11 | Atlantique sud | Landes, Galice, Portugal du Nord | 1 |
| 12 | Plaine pannonienne-danubienne | Hongrie, Valachie | 2 |
| 13 | Forêt mixte hémiboréale | Scandinavie du Sud, Baltique, Pologne du Nord | 5 |
| 14 | Méditerranéen sec (maquis égéen) | Grèce, Dalmatie, Sardaigne, Chypre | 3 |

Zones humides et sables côtiers restent des couches azonales superposées.

Consommateurs à adapter (tout tableau indexé par biome accepte 15 classes, repli sur `parent`) :
crate `core/crates/vegetation` (`species.rs`, `stands.rs`, `lib.rs`), pont
(`vegetation_scatter.rs`, `battle_terrain_field.rs`, `battle_terrain_kernel.rs`), scripts
`game/scripts/battle/battle_terrain.gd` et `game/scripts/map/{countryside_layer, field_layer,
rock_outcrops, hb_ground, map_bird_flocks, tree_species, vegetation, vegetation_mask}.gd`, carte de
couleur, `data/art/ground_biome_mix.json`, `tree_species`. Chaque nouveau biome a une entrée dans
ces données.

### 2b. Campagne

- **Fond** : les 7 couches globales deviennent ~3 rôles par biome (herbe/sol nu, sous-bois,
  roche locale), soit ~40 couches ; neige, sable, éboulis partagés. Une table (biome × rôle →
  couche) dans les données ; le shader fond deux biomes sur ~10 km aux frontières.
- **Parcellaire HB** : les 27 matières régénérées en local en 2k + ~15 nouvelles (oasis,
  palmeraie, dunes, toundra à lichens, taïga claire, lœss, landes de pins, maquis…).
- **Mémoire** : ~80 couches, albédo 2k / normale 1k ≈ 280 Mo. Réglage « Qualité des textures :
  haute / moyenne » dans Réglages ; « moyenne » ignore le niveau 2k au chargement (≈ 70 Mo).

### 2c. Bataille

- 13 **rôles** (herbe, prairie, sous-bois, terre battue, boue, galets, roche, labour, neige,
  prairie fleurie, herbe piétinée, chaume, labour frais) ; chaque biome fournit une matière par
  rôle ou en hérite d'un autre biome.
- Seul le biome du lieu est chargé : 13 couches 2k/2k ≈ 75 Mo.
- `data/fx/battle_ground_layers.json` devient « rôle → matière par biome » ; `terrain_tints`
  disparaît.
- Micro-détail fondu sous ~30 m de distance caméra.

Volume des sols : ~160 matières (une partie partagée).

## 3. Végétation

Textures seulement ; modèles 3D d'arbres inchangés mais branchés sur les nouvelles écorces et
feuillages quand l'essence correspond.

- **Cartes au sol** : ~5 par biome, ~70 cartes, 1024 px avec alpha (au lieu de 512). Exemples :
  tamaris, halfa (désert) ; lichens, camarine (toundra) ; stipe (steppe) ; bruyère, ajonc
  (Atlantique sud) ; myrtille (hémiboréal) ; ciste, lentisque (maquis) ; armoise (semi-aride).
- **Herbe de bataille** (`grass_blades`, `grass_tufts`, `grass_clump`) : une version par groupe
  (vert, sec, steppique, alpin, arctique), choisie selon le biome de la bataille.
- **Feuillages et écorces** : planches de feuilles 2k et écorces raccordables 2k pour ~12
  essences (chêne, hêtre, bouleau, pin sylvestre, épicéa, olivier, chêne vert, pin d'Alep,
  palmier dattier, peuplier, saule, mélèze) ; chaque essence de `tree_species` pointe vers ses
  textures.

Volume : ~100 images.

**Eau** : reste procédurale (Z-Image ne produit pas de normales d'eau utilisables) ; les
textures de `game/assets/textures/water/` passent de 512 px à 2k par le générateur procédural
existant, sans nouvelle image générée.

## 4. Bâtiments

- Les ~11 rôles de `BuildingMaterials` restent ; chaque région de `data/art/building_regions.json`
  gagne un champ facultatif `materials: {rôle: id}`. Sans ce champ : matières par défaut.
- `data/art/building_materials.json` garde le rendu des rôles (tuile, teinte, rugosité) ; les
  variantes régionales vivent dans le catalogue `data/art/textures/building_materials.yaml`.
- ~60 matières, albédo 2k / normale et rugosité 1k. Exemples : brique baltique et flamande,
  brique et pierre lombarde, pisé et adobe, terre crue, rondins, calcaire blond, granit gris,
  tuf, badigeon blanc, pierre volcanique ; bardeaux, lauzes, chaume de roseau, toit de tourbe,
  terrasse de terre, tuile canal ; colombages normand, alsacien, anglais (composites à partir des
  nouveaux enduits et bois).

## 5. Erreurs, tests, coût

- Échec mflux : consigné, le lot continue ; ratés listés au manifeste.
- Contrôle raté : graine + 1, deux fois au plus, puis `flagged` (signalé sur la planche).
- Matière manquante au chargement : matière de repli du rôle, sans plantage.
- Tests : `pytest` par module de la fabrique (erreur de bord sous seuil, forme et plage PBR,
  schémas des catalogues, cohérence manifeste ↔ paquet) ; `cargo test` (repli des biomes) ;
  `smoke.gd` ; scripts `*_shot.gd` par biome en campagne et en bataille, au plus 10 captures de
  contrôle par sous-chantier.
- Coût : 0 $. Temps GPU : ~330 images × ~1 min + agrandissement ≈ 6 h, en arrière-plan, par lots,
  machine calme.
- Chaque tranche (1, 2a, 2b, 2c, 3, 4) se fusionne seule dans `main` ; note de suivi
  `docs/wip/tx-textures-regionales.md`.
- Fin : partie pilote du joueur ; suppression des textures Poly Haven, ou maintien pour la
  bataille si le rendu local y est inférieur.
