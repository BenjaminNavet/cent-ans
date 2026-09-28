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
- [ ] 2 régénération complète relancée après correctifs (build → horizon, journal scratchpad).
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
- Valaam, Djerba, Gozo, Poide (Ösel) : `port: true` (îles sans autre accès) ; Altefähr (bac de
  Rügen, port) ajoutée : Bergen de Rügen n'était plus isolée sans port.
- Colonies à plus de 40 km de leur province (Voronoï) : Borgå → Tavastie, Goslar et Hildesheim →
  Brunswick, Pantelleria → province de Tunis avec `owner: fac_sicily` (îlot rattaché à la côte
  africaine par la règle des îlots), Brzeg → Haute-Silésie avec `owner: fac_legnica` (enclave :
  le duché de Wrocław sépare Legnica et Brzeg) + Chojnów ajoutée à Legnica ; Luna : poids
  Voronoï 0,4 → 0,7 (Illueca).
- Restent ramenées de loin (> 40 km), à revoir : Volok Lamski (Torjok, enclave novgorodienne
  au-delà de Tver), Gorokhovets (Vladimir), Kamianiets (Hrodna, en fait terre de Brest ; déplacer
  demande une 3e localité attestée pour Hrodna).

## Bug trouvé : dépressions sous le niveau de la mer
La terre ≤ 0 m est de l'eau pour le jeu (plan d'eau Y = 0, `relief_shade.upsample_sign`,
navgrid) : toute la dépression caspienne (−28 m, delta de la Volga, Saraïtchik), la Qattara, les
chotts, le Jourdain étaient de la mer. `terrain.lift_inland_depressions` (appelé par
`geo build`) remonte à 0,5-1 m la terre ≤ 0 m non reliée à l'océan (graine : golfe de Gascogne) ;
les basses terres reliées à la mer (Pays-Bas) gardent le comportement d'avant. Test
`tools/tests/test_inland_depressions.py`.

## Prochaine étape
Fin de `geo build`, contrôles (colonies hors province, provinces sans pixel, îles sans port),
puis splat, relief-shade, navgrid, rivers-render, horizon.
