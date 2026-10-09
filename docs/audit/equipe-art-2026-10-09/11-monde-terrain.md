# World / terrain artist — état des lieux (09/10, lecture seule)

## 1. État actuel
- **Relief** : pyramide `data/map/pyramid/` 7 étages, ~19 800 tuiles (4,7 Go). E1-E2 GLO-90 sur toute l'emprise ; E3-E4 GLO-30 sur le cœur France/Angleterre/Bénélux seulement ; E5-E7 MNT nationaux (`detail_zones.json`). Manifeste `relief_pyramid.json`, hébergement `relief_hosting.json`. Quadtree natif obligatoire (ADR 0203, crate `relief-lod`), `terrain.gdshader` 930 l. + ~20 inclusions, occlusion de vallée (ADR 0168).
- **Biomes** : `geo/biomes.py` → `data/map/biomes.png` (7 biomes Köppen 1901-1930) ; `hb_ground.gdshaderinc` (27 matières) ; 23 essences en imposteurs ; rochers ; massifs et zones humides (HC5) ; eaux douces (DN ME4).
- **Eau** : côtes par géologie (ADR 0163), mers par bassin, rivières fines CAFV avec lit creusé et passages.
- **Deux vues** (ADR 0124) : 3D puis parchemin au-delà de d = 1200.
- **Horizon** : panoramas EP2 en bataille seulement ; la campagne finit en brume.
- **Outils** : 50 modules `tools/cent_ans_tools/geo/` + tests ; `geo relief-update` (ADR 0149).

## 2. Forces
- Donnée réelle et propre (MNT sans sursol, anachronismes effacés, hydrographie recalée).
- Chaîne reproductible et versionnée ; une seule voie de rendu (~1 000 l. de repli supprimées).
- Relief alpin lisible « façon Total War » (`docs/img/dn/survey/3_alpes.png`) ; budget de génération faible (HB ~3,8 $, RC5 ~0,3 $).

## 3. Faiblesses
1. **Paquet de relief v3 non commité** : `relief_pyramid.json`, `relief_hosting.json`, `rivers_fine.json`, `fine_anchors.json` modifiés dans `main` depuis le 02/10 ; étapes 6-7 de `docs/wip/relief-auto.md` ouvertes. Risque : un autre poste ou le lanceur Windows télécharge v2.
2. Mer : motif de vagues en damier au large, tirets rectilignes (`2_bretagne_atlantique.png`).
3. Teinte rouge-orangé sur l'Angleterre au loin (faction ou brouillard qui fuit ?).
4. Verts saturés, « gazon » ; bocage peu lisible.
5. Alpes uniformément beige-blanc, `snow_line_m` fixe à 2 750.
6. Relief fin limité au cœur franco-anglais ; Rhin/Espagne/Italie à 90-180 m, passages non recalés.
7. Hydrographie moderne : lacs historiques absents, anomalie −12 m au sud de Londres.
8. Terrain ≈ moitié de l'image (15,9 → 8,1 ms masqué).
9. Semis de végétation 300-800 ms par tuile en GDScript.
10. Jugements visuels RC3, HB5, HB7, HC3 jamais faits.
11. Altitude plafonnée à 4 800 m (Caucase écrêté).
12. Pas d'horizon en campagne.

Points 2-5 jugés sur deux captures seulement, à confirmer en jeu.

## 4. Améliorations
| # | Action | Impact | Effort | Coût | Dépend de |
|---|---|---|---|---|---|
| P0 | Finir RA : vérifier/publier Release v3, commiter les données, ADR, `docs/geo.md` | Évite un relief cassé ailleurs | S | 0 | release |
| P1 | Mer : casser le damier, traquer les tirets (`water.gdshader`) | Fort | S-M | 0 | tech art |
| P1 | Diagnostic de la teinte rouge lointaine | Fort | S | 0 | UI carte |
| P1 | Étagement alpin (alpage, éboulis, roche, neige selon latitude/saison) | Fort | M | 0 | DA |
| P1 | Coût pixel du terrain (variante lointaine, couches fusionnées dans la colormap) | Fort (FPS) | M-L | 0 | tech/perf |
| P2 | Verts désaturés par biome/saison, bocage lisible | Moyen-fort | S-M | 0 | DA |
| P2 | Semis de végétation en Rust ou cuit hors ligne | Moyen | M | 0 | moteur |
| P2 | Planche de jugement HC3/RC3/HB5 + banc par biome | Moyen | S | 0 | QA, DA |
| P2 | Lacs et zones humides historiques | Moyen | M | 0 | game design, historien |
| P3 | Horizon de campagne en vue rasante | Moyen | M | 0-0,5 $ | tech art |
| P3 | Étendre E3-E4 au Rhin/Espagne/Italie | Moyen | L | 0 (+ Go) | hébergement |
| P3 | Plage d'altitude à 6 000 m | Faible | S (+ recuisson) | 0 | hébergement |
| P3 | Corriger −12 m de Londres, bras de 1340 | Faible-moyen | M | 0 | historien |

Ordre : P0 sans attendre, puis les P1 visuels ; coût pixel en parallèle avec la tech.
