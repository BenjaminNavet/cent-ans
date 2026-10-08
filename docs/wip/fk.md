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
FK3 : clés de `map_scenes.json` lues par le rendu à aligner avec le schéma FK1 (voir docs/archive/chantiers.md).

## Points ouverts vague 2
- FK4 : fumées de scène (feux LifeEffects) visibles jusqu'au palier moyen ; crue = plaque opaque, pas de gens sur les toits ; fuyards en ligne droite ; orientation étals/échafaudage à vérifier.
- FK5a : clic sur la scène non branché (seul le sceau ouvre la décision) ; pictogramme unique ; avis d'expiration lié au libellé « (délai écoulé) » (`EXPIRED_MARK`).
- FK5b : taux 0,317/tour (cible atteinte, test #[ignore] 165 s) ; graine de `f7_events::montereau…` passée à 6 ; bateliers sans condition fleuve ; pas d'entrées codex.

## FK6 (fait 09-29/30)
- Vérifs complètes vertes sur integration/fk après fusion de main (cargo fmt/clippy/test, pytest 1277, smoke, fk_folk, fk2_assets, fk5_incidents).
- A/B Paris (2213,3204 cadre monde ; anciennes coordonnées 2213,1924 = avant ADR 0121), d=45 : avant correctifs 13,9 vs 14,5 i/s (−4,4 %) ; après correctifs mesure bruitée (ollama actif), paire propre 145 vs 145 i/s ; +78 appels de rendu, ≈ 436 figurines. Le worktree d'intégration a besoin du lien `data/map/pyramid` vers le cache de main.
- Correctifs FK6 : préchauffage étalé (plus d'à-coup de 200-500 ms), caches par tour (placement 3-10 ms), emprises des villes emblématiques exclues, taille minimale à l'écran (`figure_min_view_fraction` 0,045, `figure_height` 1,26 : 8-21 px à d=12), capture qui attend `FolkPool.settled()`, `--fps-probe` imprime `folk`.
- Captures : docs/audit/captures/fk/ (6/6 ; `paris_plague_close_final.png` = état final).

## Reste ouvert (pour la partie pilote)
- Scène de peste à d=12 peu lisible sur la capture finale (figurants surtout sur les routes) : juger en jeu.
- Placement à d=45 : 9-10 ms (cible 8).
- Fumées de scène visibles jusqu'au palier moyen ; crue = plaque opaque ; fuyards en ligne droite ; clic sur la scène n'ouvre pas l'incident (seul le sceau) ; pictogramme d'incident unique.
- Bateliers sans condition fleuve ; pas d'entrées codex pour les 15 événements ; test de taux #[ignore] (165 s, 0,317/tour).
- A/B à refaire sur machine calme.
