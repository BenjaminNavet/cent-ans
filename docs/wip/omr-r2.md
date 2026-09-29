# OMR-R2 — chargement et mémoire de la carte de campagne

Lot R2 de `docs/wip/omr.md`. Worktree `../gp-omr-r2`, branche `feat/omr-r2`.
Cible : chargement réel ≤ 7,5 s (OM : 11,4 s), RSS max ≤ 2,2 Go (OM : 2,89 Go).

## Protocole
- Sonde `game/tests/r2_load_probe.gd` (réglages de test, `user://settings.cfg` intact) :
  `CENT_ANS_RELIEF_DIR=/Users/jean_hubert/dev/game_project/data/map /usr/bin/time -l godot --headless --path game --script res://tests/r2_load_probe.gd`
  `wall_ms` = instanciation → `load_ok` et `ReliefLandcover.pending()` faux ; RSS = `time -l`.
- dylib : `core/target/i1/libcent_ans.dylib` (aucun changement de `core/` depuis b9e8a069b).

## État
- [ ] mesure de référence

## Mesures (machine partagée, bruit ±1 s)
| étape | wall | total_ms | RSS max | statique |
|---|---|---|---|---|
| référence (8e31dbf) | 13,7-14,0 s | 11 220-11 490 | 2,91-3,20 Go | 2 098 Mo |
| croissance CV1 groupée | 10,8 s | 8 406 | | 2 099 Mo |

Décomposition de référence (wall 14,0) : avant `_ready` (scène + `_ready` des enfants) 1,8 s ;
données 0,9 ; terrain 2,2 ; décor 2,9 (colonies 1,6, rivières 1,1) ; `refresh_all` 4,0 dont
`CampaignLife.refresh` 3,5 (517 maquettes remplacées une à une, chacune suivie de passes sur
toutes les colonies : paires, masquage, hauteurs d'étiquettes) ; 1re image 0,7.
`relief_landcover` (fil de travail) : 2,3 s de décodage seul, mais posé à la 1re image (après
`_ready`), d'où les 7,8-9,7 s affichés.

## État
- [x] mesure de référence
- [x] `SettlementLayer.replace_models` (lot) : croissance 3 375 → 109 ms
- [x] relief_shade en BC5 (RGTC RG) + mipmaps précalculés : `tools/cent_ans_tools/geo/bc5.py`,
  `cent-ans geo relief-shade-bc5` (depuis les bandes PNG), 8 parts zlib `relief_shade_bc5_<i>.bin`
  (≈ 16 Mo chacune, 131 Mo au total), `map.json.relief_shade.bc5` ; `ReliefLandcover.load_bc5`
  (repli PNG) ; shader `relief_shade_rg` (occlusion en .g). Texture 447 → 224 Mo. Test
  `r2_relief_bc5_test.gd` (erreur moyenne 0,8 niveau, max 18 ; borne BC4 = étendue / 14).
- [x] `ModelLibrary.tint_banner` : matériau surchargé par instance au lieu d'un maillage dupliqué
  par couleur (312 maillages copiés, ≈ 350 Mo de sommets en rendu réel ; sans effet en headless,
  le rendu factice ne garde pas les sommets).
- [x] Sommets lointains : colonnes/lignes précalculées, grilles du fond en local (identique au bit,
  `r2_chunk_vertices_test.gd`, 2,5 × plus rapide ; 1,4 s → 0,05 s en jeu, la contention du
  relief ayant disparu).
- [x] Rivières : tronçons en parallèle (`_piece_arrays`, tableaux dimensionnés d'avance).
- [x] Zones humides en BC1 (`wetlands_bc1_0.bin`, 0,3 Mo ; 126 → 22 Mo ; test BC1 dans
  `r2_relief_bc5_test.gd`) ; module renommé `block_compress.py`, commande `geo gpu-textures`.
- [x] Fond de relief : min/max sur valeurs brutes 16 bits (identique, `r2_relief_floor_test.gd`).
- [x] ADR 0118 (numéro à vérifier à l'intégration), docs/geo.md.
- [ ] smoke.gd + tests carte (29 verts sauf da7d : seuil de temps de `declutter` 11 ms sous charge
  50, à relancer au calme), A/B alterné (`scratchpad/ab.sh` : A = scripts de 8e31dbf, B = HEAD).

## Prochaine étape
Mesure A/B au calme, puis rapport. Pistes restantes si la cible n'est pas atteinte : colonies
(1,0 s, 1 200 maquettes instanciées puis 517 remplacées par la croissance CV1), `FineGeoLayer.setup`
(0,5-0,7 s), première image (0,7 s), chargement de la scène (1,5 s de compilation GDScript).
