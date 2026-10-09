# DN : reprise locale des assets rejetés

Mandat : « tous les assets rejetés doivent être réessayés avec le modèle local ». Zéro dépense fal.
Liste : 27 refusés D5 + econ_sheep_shearing_pen (filtre fal). Branche `dn/local-retry`.

## Procédé
1. Anciens artefacts fal rangés dans `~/dev/cent-ans-raw/dn/<id>/fal_rejected/`.
2. Images Z-Image Turbo local (mflux), 3 graines, `dn_batch.py --image-backend local --seeds 3 --until select --charter warn`.
3. Choix : meilleure graine conforme strict ; sinon meilleure en `warn` (scores notés dans production.md).
4. 3D : TRELLIS HF si quota, sinon SF3D local ; classes orientées : multi-vue si voie gratuite.
5. `cent-ans dn-ingest`, manifeste, galerie.

## État (EN PAUSE, demande du joueur)
- Fait : repli local automatique dans `tools/experiments/dn_batch.py` (3D, image, multivue Qwen) + test + doc pipeline § 7 ; `tools/experiments/dn_cards.py` (textures card_*, non lancé).
- Images locales (3 graines, `chosen.json` = 1re graine, charte `warn`) : cart_relic_procession, pack_camel_bactrian_laden, pack_camel_laden, ship_galley_aragonese, ship_galley_genoese. Pas de 3D, pas d'ingest. Anciens artefacts fal de ces 5 ids : `dn/<id>/fal_rejected/`.
- Restants : 22 refusés (nature_extra 14 dont crops/arbres, map_extra 3, architecture 2, economy saltpan, battle_extra tente) ; econ_sheep_shearing_pen abandonné (existe) ; 21 figures jamais générées ; 14 cartes `card_*`.
- Les 22 autres ids ont leurs dossiers fal d'origine intacts (rangement annulé).

## Reprise
Depuis le worktree `gp-dn-local-retry` (ou main une fois fusionné), sans FAL_KEY, GPU libre :
```sh
R="uv run --with rembg --with onnxruntime --with pillow --with numpy python tools/experiments/dn_batch.py"
# 1. refusés (images locales, 3 graines) ; ranger d'abord dn/<id>/{img,cut,cut_raw,scores.json,contact.png,chosen.json,generation.json,3d,views} dans dn/<id>/fal_rejected/
$R data/art/dn_catalog_nature_extra.json --only <ids> --seeds 3 --image-backend local --charter warn --until select --backend3d sf3d
# idem map_extra, architecture, economy, battle_extra, mobile (cf. production.md)
# 2. choix par l'œil (chosen.json {"seed":N}) puis 3D : relancer sans --until (hf puis sf3d ; --backend3d hf+sf3d), classes orientées : repli Qwen back view
# 3. figures : catalogues battle (archer_3_jack, archer_5_handgunner, cavalry_1_druzhina, cavalry_4_routier, cavalry_5_jinete, standard_0_bearer), campaign (army_lord_mounted), mobile (14 fig_*), mêmes options
# 4. cartes : uv run --with rembg --with onnxruntime --with pillow --with numpy python tools/experiments/dn_cards.py
# 5. cent-ans dn-ingest (voir docs/pipeline-assets-3d.md) ; les glb de game/assets/models/dn restent hors git (ADR 0212)
```
