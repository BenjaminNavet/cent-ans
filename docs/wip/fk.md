# FK — carte vivante (gens, marchands, scènes, incidents)

Spec : `docs/design/2026-09-29-carte-vivante-folk.md`. ADR : 0122 (expiration → option de l'IA).
Coût cloud : 0 $.

## État
- [x] FK0 squelette : `map_scenes.rs` (API vide), `tests/fk_map_scenes.rs` (#[ignore]),
  `game/scripts/map/life_folk/*.gd` vides, `game/tests/fk_folk_test.gd` (SKIPPED), ADR 0122.
- [x] Vague 1 : FK1, FK2, FK3 fusionnés dans integration/fk (299233c8c), modèles FK2 rebranchés (234c1e845).
- [x] Vague 2 fusionnée dans integration/fk (5509b4ff2 ; conflit trivial campaign_life.gd résolu). Lancée depuis integration/fk : FK4 scènes + alignement des clés map_scenes.json (feat/fk4-scenes), FK5a marqueurs/fenêtre/notification (feat/fk5-incidents), FK5b 15 événements + taux (feat/fk5-events).
- [ ] FK6 : A/B `--no-folk`, captures (≤ 3), relecture, fusion.

## Intégration
Worktree `../game_project-fk`, branche `integration/fk` : FK2 + FK3 fusionnés (25273ae81). Rebranchement des modèles FK2 dans `FolkModels` en cours (figurines `villager_*`, accessoires face +X). FK1 (feat/fk1-core) : taux d'incidents mesuré 0,147/tour (sous la cible 0,25-0,5 ; FK5 ajoute 15 événements).
FK3 : clés de `map_scenes.json` lues par le rendu à aligner avec le schéma FK1 (voir docs/wip/fk3-folk.md).

## Points ouverts vague 2
- FK4 : fumées de scène (feux LifeEffects) visibles jusqu'au palier moyen ; crue = plaque opaque, pas de gens sur les toits ; fuyards en ligne droite ; orientation étals/échafaudage à vérifier.
- FK5a : clic sur la scène non branché (seul le sceau ouvre la décision) ; pictogramme unique ; avis d'expiration lié au libellé « (délai écoulé) » (`EXPIRED_MARK`).
- FK5b : taux 0,317/tour (cible atteinte, test #[ignore] 165 s) ; graine de `f7_events::montereau…` passée à 6 ; bateliers sans condition fleuve ; pas d'entrées codex.

## FK6 (en cours, integration/fk 1ed3d8894)
- Vérifs complètes vertes après fusion (cargo fmt/clippy/test, pytest 1275, smoke, fk_folk, fk2_assets, fk5_incidents).
- A/B Paris (2213,3204 cadre monde ; anciennes coordonnées 2213,1924 = avant ADR 0121), d=45, 3 paires : 13,9 vs 14,5 i/s (−4,4 %, cible ≤ 5 %), 463 figurines + 149 accessoires, +25 appels. Le worktree a besoin du lien `data/map/pyramid` vers le cache de main.
- Défauts : placement FolkScenes 100-310 ms (à ramener sous 8 ms) ; à d=12 sur Paris, aucune figurine visible malgré 121 posées. Agent cent-ans-dev en correction dans integration/fk. Captures (3/3 utilisées) : docs/audit/captures/fk/.

## Prochaine étape
FK6 : vérifs complètes sur integration/fk, A/B `--no-folk`, 3 captures, puis fusion dans main.

## Ancienne étape
Lancer la vague 1 (branches `feat/fk1-core`, `feat/fk2-assets`, `feat/fk3-folk`), intégration
dans `integration/fk`.

## Points ouverts
- FK2 fini (feat/fk2-assets, a31bbf3a9) : 15 .glb + manifest.json, clips scythe/carry/plough (rig fin, 61 clips), figurines `villager_0..3` ; note docs/wip/fk2-assets.md. À vérifier en capture : faux vs jambe gauche, sac vs tête, torche non émissive, lisibilité à l'échelle de la carte ; rig grossier sans les 3 clips (repli marche/attente).
