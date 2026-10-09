# NA — codex du décor naturel + bulle de survol (ADR 0217)

Worktree `../gp-na`, branche `feat/na`. Demande joueur 2026-10-09 : le codex couvre-t-il arbres,
faune… ; survol d'un élément du décor → petite bulle après quelques secondes.

## Lots
- [x] NA0 squelette : schéma (`decor`, `latin`, catégories), validateur, `CodexStore.entry_for_decor`, famille Nature, ADR.
- [ ] NA1 fiches (3 agents Sonnet) : arbres / animaux / oiseaux+roches, liste ci-dessous.
- [ ] NA2 pick campagne : arbres (instances MultiMesh de `map/vegetation.gd`, espèce dans custom data), troupeaux (`fauna_layer.cell_herds`), rochers.
- [ ] NA3 pick bataille : arbres par espèce (`battle_terrain.gd` by_species).
- [ ] NA4 bulle différée (1,5 s, réglage Réglages, 0 = off) + test headless.
- [ ] NA5 bundle, validate, smoke, revue, fusion main.

## Fiches à écrire (id → décor)
Arbres (catégorie `arbre`) : cdx_chene (tree:oak, battle_tree:oak), cdx_hetre (tree:beech, battle_tree:beech),
cdx_sapin (tree:fir), cdx_erable (tree:maple), cdx_chataignier (tree:chestnut), cdx_bouleau (tree:birch),
cdx_epicea (tree:spruce), cdx_pin_sylvestre (tree:scots_pine), cdx_pin_maritime (tree:maritime_pine),
cdx_pin_parasol (tree:stone_pine), cdx_chene_vert (tree:holm_oak), cdx_olivier (tree:olive),
cdx_cypres (tree:cypress), cdx_peuplier (tree:poplar, battle_tree:poplar), cdx_pommier (tree:apple, battle_tree:fruit),
cdx_meleze (tree:larch), cdx_pin_d_alep (tree:aleppo_pine), cdx_pin_noir (tree:black_pine),
cdx_chene_kermes (tree:kermes_oak), cdx_lentisque (tree:lentisk), cdx_arbousier (tree:arbutus),
cdx_frene (battle_tree:ash), cdx_haies_et_buissons (battle_tree:bush).
Existantes complétées : cdx_saule (tree:willow, battle_tree:willow), cdx_genievre (tree:juniper).

Animaux (`animal`) : cdx_bovins (fauna:animal_cattle_red, animal_cattle_pied, animal_ox_draught),
cdx_taureau_de_camargue (animal_camargue_bull), cdx_cheval_de_trait (animal_horse_draught, animal_camargue_horse),
cdx_tarpan (animal_horse_wild_steppe), cdx_mouton (animal_sheep_wool, animal_sheep_merino), cdx_chevre (animal_goat),
cdx_porc (animal_pig_forest), cdx_ane_et_mulet (animal_donkey, animal_mule), cdx_chameaux (animal_camel_dromedary, animal_camel_bactrian),
cdx_cerf (animal_deer_red_stag, animal_deer_hind), cdx_chevreuil (animal_roe_deer), cdx_sanglier (animal_boar_wild),
cdx_ours (animal_bear_brown), cdx_bouquetin (animal_ibex), cdx_chamois (animal_chamois), cdx_renne (animal_reindeer),
cdx_elan (animal_elk), cdx_aurochs (animal_auroch), cdx_bison (animal_bison_european), cdx_renard (animal_fox_red),
cdx_lievre (animal_hare), cdx_castor (animal_beaver), cdx_phoque (animal_seal_grey), cdx_morse (animal_walrus).
Existante complétée : cdx_loups_et_louveterie (animal_wolf_grey).

Oiseaux (`oiseau`) : cdx_corneille (bird:crow), cdx_oie (bird:goose), cdx_etourneau (bird:starling),
cdx_goeland (bird:gull), cdx_flamant_rose (bird:flamingo), cdx_grue (bird:crane), cdx_cigogne (bird:stork),
cdx_rapaces_et_fauconnerie (bird:raptor).
Roches (`roche`) : rock:limestone_cliff, granite_chaos, stratified_ridge, alpine_spire, red_sandstone, mossy_erratic
(ids libres, ex. cdx_falaises_calcaires, cdx_chaos_granitiques…).

## État / prochaine étape
NA0 commité. Suite : lancer NA1 (agents) et NA2-4.
