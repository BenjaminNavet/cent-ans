# FK3 — réservoir de figurines, vie ordinaire, marchands (branche `feat/fk3-folk`)

Spec : `docs/design/2026-09-29-carte-vivante-folk.md` §§ 2.2, 3.1, 3.2, 5-7. Suivi général : `docs/wip/fk.md`.

## État : terminé (à intégrer dans `integration/fk`)
- `life_folk/folk_models.gd` (`FolkModels`) : table rôles → figurines skinnées (candidats, FK2 en
  tête : `civilian_0..3`), activités → clips (`plough`, `scythe`, `carry` de FK2 pris dès qu'ils
  existent), accessoires → `res://assets/models/folk/*.glb` sinon maquettes en boîtes.
- `life_folk/folk_pool.gd` (`FolkPool`) : un `MultiMesh` par (rôle, activité) + un par accessoire,
  plafond `pool_cap` (repli 600, accessoires : un quart), moitié en dépassement de budget, palier
  proche seulement, rayon autour du point visé, placement différé jusqu'à caméra posée (≤ 0,5 s).
- Déplacement en shader : `#ifdef FK_TRAVEL` dans `battle_soldier_skinned.gdshader` (variante
  compilée pour la carte seulement) et `shaders/folk_prop.gdshader`.
- `folk_routine.gd` (§ 3.1) et `folk_caravans.gd` (§ 3.2) ; branchés dans `CampaignLife`
  (`--no-folk`, `--folk-off=routine,caravans,...`).
- `game/tests/fk_folk_test.gd` vert, `smoke.gd` vert, shaders compilés sans erreur (lancement GPU).

## Mesures (Paris, distance 45)
250-430 figurines, placement 4-12 ms après le premier (création des groupes ≈ 50 ms, une fois).

## Prochaine étape
FK4 (scènes) : fournisseur enregistré avant les marchands (`folk.register` dans
`CampaignLife._setup_folk`). FK6 : A/B `--no-folk`.

## Points ouverts
- Trajets rectilignes en boucle (pas de suivi de polyligne en shader) : une charrette parcourt un
  tronçon ≤ 6 unités puis réapparaît au début (masqué par un rétrécissement).
- Clés lues dans `map_scenes.json` (premier niveau, `folk` ou `densities`) : `pool_cap`,
  `activity_radius`, `figure_height`, `road_folk_per_unit`, `field_work_probability`,
  `herd_probability`, `woodcutter_probability`, `pilgrim_probability`, `carts_per_value`,
  `guard_value`. À aligner avec le schéma FK1.
