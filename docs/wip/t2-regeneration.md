# T2 — régénération des 459 modèles en TRELLIS.2 local + paquet models-v3

Décision : ADR 0342 (adoptée par le joueur le 11/10). Procédé : `docs/pipeline-assets-3d.md` § 5.
Essai d'origine : `docs/wip/tp-peinture-texture.md`. **Mandat du joueur : générer les 459 et publier
`models-v3`** (feu vert donné le 11/10, pas besoin de redemander).

## Périmètre
- 459 entrées de `data/art/dn_manifest.json` dont les fichiers sont dans `dn/buildings`, `dn/props`,
  `dn/siege` (house 132, major_building 128, prop 138, cart 27, field 11, bridge 4, siege_engine 19).
- Hors périmètre : arbres (imposteurs), animaux, rochers, navires.
- **Décision 11/10** : les 6 sans image sont exactement les 6 déjà en TRELLIS 2 (fal, `fal2__s*.glb`) → gardés tels quels, **453 à générer**.
- 453 ont leur image source (`source_image`, `~/dev/cent-ans-raw/dn/<id>/cut/s<graine>.png`).
  **6 sans image** : bombard, cart_litter_noble, debris_ram_wreck, econ_saltpan_mediterranean,
  econ_terrace_olive_grove, siege_handgonne_pair → régénérer l'image (Z-Image, prompt dans
  `dn/<id>/prompt.txt` s'il existe) ou les laisser en TRELLIS 1.
- 24 étaient en TRELLIS 1 multivue (maquettes de villes) : TRELLIS.2 n'a qu'une image → vérifier
  leur dos sur la planche ; si pire, garder l'ancien.

## Plan
1. **Script de lot** `tools/experiments/t2_batch.py` (à écrire) : pour chaque id du périmètre, un seul
   processus Python qui charge TRELLIS.2 une fois (importer le pipeline de `trellis-mac/generate.py`
   au lieu de 459 lancements, sinon ~1 min de chargement par objet) ; sortie
   `~/dev/cent-ans-raw/dn/<id>/3d/trellis2__s42.glb`, supprimer l'`.obj` ; reprise possible (sauter
   les glb déjà présents) ; journal + `failures.jsonl` ; sous `tools/gpu_lock.sh`.
2. **Ingestion** : `cent-ans dn-ingest <brut> --id <id> --class <classe> --<axe> <dimension actuelle>
   --yaw <yaw_deg actuel> --source-image … --model-3d trellis2-local --cost-usd 0` (reprendre classe,
   dimensions et lacet de l'entrée de manifeste existante pour garder l'échelle en jeu). Puis
   `fix_inverted_faces.py` sur les 3 LOD.
3. **Contrôle** : `dn_back_check.py` sur les 459 (comparer au nombre de `dark_back` : 18) ; planches
   de comparaison avant/après sur un échantillon (château, abbaye, poterne, 3 maisons, 1 ville
   multivue, 1 engin) ; ne refaire qu'en cas d'erreur flagrante (règle du joueur).
4. **Jeu** : `godot --headless --path game --import`, puis `smoke.gd` ; un coup d'œil (≤ 5 captures,
   via `tools/godot_bg.sh`, sans voler le focus).
5. **Publication** : commiter `data/art/dn_manifest.json` ; `uv run --project tools cent-ans art
   models-update` (publie `models-v3`) ; commiter `data/art/dn_models_hosting.json` ; push.

## Contraintes
- Temps : 140-170 s/objet génération → ~20-23 h machine pour 459 : lots de nuit, en arrière-plan.
- Mémoire : pic ~18 Go → pas d'autre charge GPU lourde (mflux, Blender lourd) en même temps.
- Disque : ~42 Go libres au 11/10 ; brut glb ~12 Mo (≈ 5,5 Go pour 459), `.obj` 80-130 Mo à
  supprimer aussitôt ; surveiller `df -h ~`.
- 0 $ : rien à consigner dans `docs/budget.md` (sauf repli payant, interdit sans accord).
- Commits avec chemins explicites, sans Co-Authored-By ; pas de glb ni de captures dans git.

## État
- [x] trellis-mac installé et banc A/B fait (11/10)
- [x] ADR 0342, doc procédé, mémoire
- [x] squelette t2_batch.py (generate / ingest) commité
- [x] implémentation t2_batch.py (generate sous gpu_lock, reprise, failures.jsonl, pas d'.obj ; ingest + fix faces)
- [x] essai sur les 3 du banc. **Piège trouvé** : le brut TRELLIS.2 pleine résolution (~180 k faces,
  milliers d'îlots UV) bloque la décimation collapse de dn-ingest, dont le repli de soudure déchiquette
  le modèle (le banc ADR 0342 n'avait comparé que les bruts). Correctif dans t2_batch : TRELLIS.2
  décime lui-même au budget LOD0 de la classe et cuit à la texture de la classe (≤ 2048). Ingestion
  alors propre, orientation identique à T1 (lacet du manifeste repris tel quel). fix_inverted_faces
  reste utile (13-70 % de faces retournées).
- [ ] génération 453 lancée le 11/10 11:40 (`nohup`, journal `~/dev/cent-ans-raw/t2/batch.log`)
- [ ] ingestion + correctif faces
- [ ] contrôle + planches
- [ ] models-v3 publié

## Prochaine étape
État dans `~/dev/cent-ans-raw/t2/` : `done.jsonl`, `failures.jsonl`, `ingested.jsonl`, journaux. Relancer
`generate` reprend où il s'est arrêté (glb présents sautés, ≥ 2 échecs sautés).

Écrire `tools/experiments/t2_batch.py` (lire d'abord `trellis-mac/generate.py` pour réutiliser le
pipeline chargé une fois), le tester sur les 3 assets du banc, puis lancer le lot complet en arrière-plan.

## 11/10 après-midi : test d'angle et de style des images sources
- Lot en pause à 22/450 (reprise : même commande `generate`).
- TRELLIS.2 est entraîné sur des rendus Blender d'assets 3D (ObjaverseXL, ABO, HSSD, TexVerse), caméra surtout 15-50° au-dessus de l'horizon (médiane 30°), lumière blanche neutre, objet seul (`data_toolkit/render_cond.py`).
- Variantes testées sur bush_hawthorn, camp_baggage_pile, abbey_west : `__v3d` (vue plongeante 35°, style photo) et `__r3d` (même vue, « clean 3D render … Sketchfab »). Catalogues : `~/dev/cent-ans-raw/t2/angle/catalog*.json`. Le joueur préfère `__r3d` ; 2 graines suffisent.
- `t2_batch.py` : `--tag`/`--image` (autre image source), classe lue dans le catalogue DN (prioritaire sur le manifeste ; un changement de classe déplace les fichiers et supprime les anciens).
- 8 végétaux passés de `prop` (1 500 tris) à `tree` (6 000 tris, débris gardés) : bush_hawthorn, hedge_bocage, env_bocage_hedge_bank, env_reed_bed_mass, crop_hemp_tall, crop_woad_yellow, crop_bean_field, crop_vine_stakes_row.
- `--budget-scale N` (dn-ingest, `t2_batch.py generate|ingest`) : multiplie les budgets de triangles de la classe, pour les essais. Essai ×2 sur camp_baggage_pile et abbey_west (tag `r3d_x2`). Aubépine en classe arbre (6 000 tris) : branches lisibles, débris de feuilles flottants.
- Page de comparaison : https://claude.ai/artifact/KSiXAnJToBYnmE9u5fKTBW. Prochaine étape : décision du joueur (régénérer ou non les images des 453 en style `__r3d`, choisir T1/T2 par modèle).

## 11/10 14:32 : relance en style « rendu 3D »
- Décision du joueur : style « rendu 3D » acquis pour toutes les générations ; T1/T2 et budget au
  cas par cas, jugés par Claude ; arbres à 6 000 tris.
- `ga3_decor.json` : style par défaut = rendu 3D. `t2_batch.py sources` écrit le catalogue
  `~/dev/cent-ans-raw/t2/r3d/catalog.json` (449 ; les 3 du banc ont déjà leur `chosen.json`).
  `town_port_harbour` n'a pas d'entrée de catalogue : à générer sur sa source actuelle (sans `--tag`).
- `t2_batch.py --tag r3d` prend la graine de `dn/<id>__r3d/chosen.json` (sinon l'asset est sauté).
- Lot : `nohup tools/experiments/t2_r3d_chain.sh` (relancer la même commande pour reprendre),
  journaux `~/dev/cent-ans-raw/t2/r3d/{chain,images,t2}.log`. ~90 s/image, ~250 s/modèle → ~2 jours.
- Ensuite : revoir les choix de graine (planches), relancer T2 sur l'autre graine si besoin, choisir
  T1 ou T2 par modèle, `t2_batch.py ingest --tag r3d`, puis étapes 4-7 ci-dessus.

## 11/10 soir : revue du paquet 1 (25 modèles)
- Correctif `dn_ingest.update_manifest` : verrou `fcntl` (les ingestions parallèles perdaient des entrées), commit 2e102e610.
- Défaut de série : TRELLIS.2 perd parfois les pièces fines ou détachées (roues, chaudrons, viande, piles de pont), 7/25. Les bruts sont déjà incomplets : ce n'est ni le filtre d'îlots de l'ingestion ni l'alpha des images ni l'export o_voxel. Test sur l'autre graine d'image (tag `alt`) : pont entier, roues et chaudrons toujours en lambeaux. L'ancien T2 (anciennes sources) avait les roues.
- Ruine en T2 (murs troués, nef effondrée) sur abbey_balt/iber/ital/nord : T1 gardé.
- Choix par asset dans `~/dev/cent-ans-raw/t2/r3d/choices.json` (`r3d`, `alt`, `t2old` = `trellis2__s42`, `t1` = modèle actuel) : paquet 1 = 13 T2, 1 ancien T2, 11 T1. À l'ingestion finale, n'ingérer que les choix T2 (option à ajouter à `t2_batch ingest` : lire `choices.json`, tag par asset).
- Revue publiée : https://claude.ai/artifact/3Nt72vVpK4vJtekFfWmoWe. Outils de revue dans le scratchpad de la session (rb1.sh : rendus séquentiels, `/tmp/ga3_compare_panel.png` partagé).
- Prochaine étape : revoir chaque paquet à sa fin, compléter `choices.json`, puis ingestion sélective.
