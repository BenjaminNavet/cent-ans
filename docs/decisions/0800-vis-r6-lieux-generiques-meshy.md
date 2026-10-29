# ADR 0800 — VIS-R6b : lieux génériques de la famille « ouest » refaits en Meshy meshy-6

Date : 2026-10-30. Statut : accepté. Lot VIS-R6b de la boucle juge 2 ; spec : `docs/audits/2026-10-29-visuel-campagne.md`
(table (d) R6, section (e), constats G2-5 et G2-6). S'appuie sur les ADR 0211, 0349, 0356, 0790, 0794.

## Contexte
Les villages, bourgs, châteaux et abbayes génériques de France, d'Angleterre et de Flandre (famille `west`) se lisaient
comme des gravats gris, et la carte en montrait trop en vue moyenne (village visible jusqu'à la distance 900, soit près de
4 000 lieux). Quatre maquettes Meshy meshy-6 texturées (≈ 1,4 M de triangles chacune, 35-52 Mo, hors dépôt dans
`~/dev/cent-ans-raw/vis-r6/meshy/`) ont été générées à partir d'images Z-Image choisies (`choix.md`). Aucun crédit Meshy
n'est dépensé dans ce lot.

## Décision
1. **Ingestion.** Nouvelle classe `settlement_maquette` (`data/art/dn_ingest_classes.json`) : budgets 30 000 / 10 000 /
   3 000 triangles, texture 2048, `voxel_bake` à 320 voxels (le brut dépasse 5 fois le budget LOD0, donc LOD0 aussi refait
   par voxels, ADR 0790), `island_min` 0 (sinon les maisons isolées sont prises pour des débris), socle gardé, sans
   étalonnage d'albédo (`--no-grade`). Mêmes identifiants que les modèles remplacés (`village_west`, `castle_west`,
   `town_west`, `abbey_west`) : la table `dn_campaign_models.json` n'a pas changé. Longueurs cibles = `native_width` de
   `town_maquettes.json` (2,0 / 1,9 / 2,0 / 2,2).
2. **Versionnement.** Les glb ingérés vivent dans `game/assets/models/dn/` (ignoré, paquet de release ADR 0212) : seul le
   manifeste `data/art/dn_manifest.json` est versionné. Le paquet n'est pas republié par ce lot (version d'hébergement
   inchangée, accord explicite requis) ; sans lui, le jeu garde les anciens modèles.
3. **Densité.** `town_maquettes.json` : visibilité du village 900 → 350. Bourg, château, abbaye inchangés.
4. **Loi d'échelle des voisins.** `collision.landmark_clearance` (10 unités) et `landmark_min_factor` (0,75) : la maquette
   d'une ville emblématique cède aussi, de façon à laisser 10 unités de sol libre jusqu'au disque du voisin non village le
   plus proche (`LandmarkPlan::yield_factor`, `places.rs`). Sans effet sur les lieux éloignés.
5. **Teintes régionales.** `dn_campaign_models.json` `tints` (schéma ajouté) : multiplicateur RGB linéaire de l'albédo par
   région, première règle gagnante : brique aux Pays-Bas (1,12 / 0,94 / 0,86), tuile au sud et au centre, ardoise à l'ouest
   et en Grande-Bretagne. Appliqué avec le gain d'albédo à la copie de matériau de `brighten_dn`. La teinte vise toute la
   maquette, pas seulement les toits (une texture unique) : les écarts restent faibles (≤ 14 %).

## Conséquences
- Tris : 30 000 / 10 000 / 3 000 par maquette, comme les anciens modèles (≈ 30 900 / 15 000 / 4 000) : coût de rendu égal ou
  moindre, texture 2048 BC1 (≈ 2,7 Mo en GPU) par modèle, inchangée en nombre.
- Moins de villages en vue moyenne : gain attendu de rendu au-delà de d 350.
- Paris : sa maquette peut rétrécir jusqu'à 25 % si un voisin non village la touche ; la plaque « Saint-Denis » reste un
  libellé d'interface projeté, donc l'écart de sol, pas l'occlusion, est ce que la loi corrige.
- Limite : les tuiles ne sont pas isolées de l'herbe dans la texture ; une vraie teinte de toit demanderait un masque.
- Écart de qualité avec les familles TRELLIS (Italie, Espagne) accru, comme prévu par l'audit.
