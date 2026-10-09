# DN — Direction artistique de nuit (08→09/10)

Mandat du joueur (08/10 soir) : revisiter les règles graphiques vers le **semi-réaliste**, produire
un maximum d'assets UI / campagne / bataille selon `docs/pipeline-assets-3d.md`, les **intégrer et
fusionner dans main** sans validation. Enveloppe fal (TRELLIS 0,02 $) : **≤ 10 $** (`docs/budget.md`).
Priorité quand le temps machine manque : **campagne > bataille > UI**. 20 agents au plus.

## Contraintes machine
- Un seul générateur local à la fois (48 Go) : Z-Image Turbo (rapide), Qwen-Image-Edit (30 min/image,
  avec parcimonie), SF3D (MPS). File de génération sérielle, agents d'intégration en parallèle.
- Ne pas toucher aux fichiers modifiés non commités d'autres sessions ni aux worktrees `gp-sc-*`.

## Vagues
- V1 audit (inventaires campagne / bataille / UI, banc d'essai du procédé, relecture de la bible).
- V2 charte révisée (bible + ADR) et catalogue d'assets priorisé.
- V3+ production (file sérielle) et intégration (worktrees `dn/*`), fusion ff-only.

## État
- [x] V1 audit : docs/wip/dn/audit-{campagne,bataille,ui}.md, revision-bible.md
- Arbitrages DA (D1-D5) : maquettes = état généralisé lointain, glb générés proche/moyen ; figurines générées via cuisson GA3 en bataille, statiques ailleurs ; budgets acceptés, 2048² bâtiments majeurs ; parchemin = enluminure ; S ≤ 0,40 contrôlé à l'image. ADR 0211 (en cours).
- V2 en cours : bible+ADR, banc procédé (dn_batch.py + verrou GPU ~/dev/cent-ans-raw/dn/gpu.lock), branches dn/ingest, dn/camp-bati, dn/fig-bake, dn/ui-codex, dn/ui-kit.
- Suite : catalogue de production (campagne d'abord : bâtiments par famille, navires, camp, ponts, folk ; puis siège/figurines ; UI : 36 événements, codex, icônes religions/ordres/édits, curseurs campagne, ornements).

- ME2/3/4/8 fusionnés f4963ebcf (faune, oiseaux, marais/névés, paysages agricoles).
- ME1 mer (10-09) : fusionnée dans main (55f48ba84), motif de vaguelettes supprimé (fondu selon la période écran, houle déformée par bruit) ; note docs/wip/dn/me1-mer.md.

## Clôture (09/10 soir)
- Tous les chantiers DN fusionnés dans main : SOL/MAQ (ADR 0213-0214), MER (0215), FLEUVE (0217), RELIEF (0218), PAYS (0220), FORET (0221), CHAMPS (0222), TROUS (8a05f8a55), villes multi-vues + champs (968ebdcf4). Aucun worktree DN restant.
- 14 villes/châteaux adoptés en multi-vues TRELLIS 1 ; 7 refusés (ancien modèle gardé) : city_iber_mudejar, city_ital_maritime, city_steppe, city_hansa, castle_west, castle_med, town_iber.
- fal : 43,51 $ dépensés (plafond 46). Règle : TRELLIS 1 (+multi) et le modèle de vues profil/dos uniquement.
- Paquet modèles `models-v2` publié (2118 fichiers, 922 Mo), `data/art/dn_models_hosting.json` → v2.
- Restes : autres villes/châteaux (castle_isl_*, castle_rus, town_isl, town_rus…), town_port_harbour encore TRELLIS 2, fragment de chaume sur le glacier, figurines archer_3_jack / fig_sled_driver_north non cuites (ga3_figures.py sur dn/fig-bake), réingestion siege_cannon_early_1340, imposteurs d'arbres (ga3_vegetation_l2.py sheet/atlas), roche HB cliff_limestone locale manquante.
