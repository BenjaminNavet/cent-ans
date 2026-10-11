# T1R — refaire les modèles DN en TRELLIS 1 (fal) avec les leçons de T2

État : **en attente du go du joueur** (11/10). Rien de lancé, 0 $ dépensé.
Origine : chantier T2 annulé (TRELLIS.2 local, `docs/wip/t2-regeneration.md`, ADR 0342 addendum du soir) ;
le joueur juge T1 meilleur presque partout. Revue T2 : https://claude.ai/artifact/3Nt72vVpK4vJtekFfWmoWe

## Leçons à appliquer
1. **Sources « rendu 3D »** (`__r3d`) : défaut de `data/art/ga3_decor.json` (préfixe « Clean 3D render of a
   single isolated », vue trois-quarts haute 35°/45°, rendu d'asset type Sketchfab). Jamais testé avec T1.
2. **Contrôle de silhouette automatique** : rendre le glb sous l'angle de la source, IoU avec l'alpha de
   `cut/s<seed>.png`. Une pièce perdue (roue, chaudron, pile de pont) fait chuter l'IoU. Seuil à caler sur le banc.
3. **Choix de graine après la 3D** : T1 sur les 2 graines d'image, garder la meilleure IoU (l'heuristique
   image de `dn_batch --select-best` s'est trompée plusieurs fois).
4. **Prompts de forme** : « intact, solid walls, complete roof » (contre l'aspect de ruine) ; pièces fines
   épaisses et attachées ; nombre d'objets contrôlé par la charte (balbal_stones : 1 stèle au lieu de 4).
5. **Dos sombres** : `fal-ai/trellis/multi` avec vues côté/dos générées (comme les villes, dos jamais noir).
6. **Ingestion** (déjà en place) : verrou du manifeste (`dn_ingest.update_manifest`, 2e102e610), `-j 3` sûr ;
   `fix_inverted_faces.py` ; `dn_back_check` ; budgets de triangles inchangés (×2 n'apporte rien).
- TRELLIS 2 sur fal reste **interdit**.

## Étape 1 : banc (≈ 10 assets, < 0,50 $)
Assets : camp_baggage_wagon_covered, baggage_wagon, camp_field_kitchen_cauldrons, camp_cooking_spit,
camp_butchery_rack, bridge_stone, bridge_gate, abbey_balt, abbey_iber, balbal_stones.
Variantes par asset :
- A : T1 actuel (manifeste) ;
- B : T1 sur source r3d, 2 graines (images déjà dans `~/dev/cent-ans-raw/dn/<id>__r3d/cut/`) ;
- C : B + `trellis/multi` (vue côté/dos).
Mesures : IoU de silhouette, `dn_back_check`, planche T1/B/C (outils de rendu : `tools/blender_scripts/ga3_compare_render.py`,
séquentiel car il écrit `/tmp/ga3_compare_panel.png`). Consigner la dépense dans `docs/budget.md`.

## Étape 2 : lot complet (si le banc convainc)
≈ 450 assets décor/bâtiments/accessoires/engins (catalogue `~/dev/cent-ans-raw/t2/r3d/catalog.json`, 449 entrées
+ town_port_harbour). Coût : 0,02 $/appel T1 → ≈ 9 $ (1 graine) à 18 $ (2 graines), plus pour `multi`.
Images gratuites (Z-Image local, ~90 s/image ; images déjà faites pour les ~50 premiers).
Garder l'actuel quand la nouvelle version n'est pas meilleure (choix par asset, cf. `~/dev/cent-ans-raw/t2/r3d/choices.json`).
Fin : ingestion, `godot --headless --path game --import`, `smoke.gd`, commit du manifeste,
`cent-ans art models-update` (models-v3), commit `data/art/dn_models_hosting.json`, push.
