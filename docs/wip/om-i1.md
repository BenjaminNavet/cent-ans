# OM I1 — intégration finale de la carte Oural–Méditerranée

Branche `feat/om-om2`, worktree `../gp-om-om2`. Profil cargo `i1`.

## Plan
1. Données D : provinces à 2 colonies, set_emba, colonies isolées sans port, autres incohérences.
2. Régénération géo complète (ordre de `om-om2.md`), commit dédié aux artefacts.
3. Points fins (fleuves, routes) : décalage `root_origin_tiles` × 256 dans le moteur.
4. Tests : cargo workspace, pytest, build.sh, Godot (smoke + tests carte).
5. Mesures : chargement, mémoire, tour d'IA.

## État
- [x] 1 (premier passage) : voir « Corrections de données ».
- [ ] 2 régénération en cours (`geo build` lancé).
- [x] 3 : `FineGeoStore` lit `root_origin_tiles` de `relief_pyramid.json` ; index et points CAFV
  en coordonnées monde (+20 tuiles E2, +1280 unités en y), chemin de fichier en coordonnées de
  cache (`CafvTile.parse(bytes, offset_tiles)`). Test : `zg5b_fine_geo_test.gd` § 2b ; test réel
  de Rouen décalé. Aucun code Rust ne lit les tuiles CAFV (rendu seul).
- [ ] 4  - [ ] 5

## Corrections de données
- 3 colonies minimum : Kubadabad (palais seldjoukide, Beyşehir), Göksun (Elbistan), Urzędów
  (Lublin), Târgoviște (Munténie, mention 1396), Floci (Munténie orientale, XVe s., incertain),
  Slatina (Olténie, 1368), Loukoml (Polotsk, 1078), Elmalı (Teke), Droutsk (Vitebsk, 1092).
- Hors carte (bord sud ≈ 29,3° N vers 20° E) : oasis de Marada, Awjila, Jalu (Ajdabiya) et Zella
  (Syrte) retirées ; remplacées par Marsa el-Brega et Zouétina (mouillages, ports, incertains) et
  Madinat Sultan (Surt médiévale).
- set_emba (53,5° E, à l'est du bord) remplacée par Jaïk (site de la Horde d'Or près d'Ouralsk).
- Valaam : `port: true` (île du Ladoga, accès par bateau seulement).

## Prochaine étape
Fin de `geo build`, contrôles (colonies hors province, provinces sans pixel, îles sans port),
puis splat, relief-shade, navgrid, rivers-render, horizon.
