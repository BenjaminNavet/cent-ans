# DN : reprise locale des assets rejetés

Mandat : « tous les assets rejetés doivent être réessayés avec le modèle local ». Zéro dépense fal.
Liste : 27 refusés D5 + econ_sheep_shearing_pen (filtre fal). Branche `dn/local-retry`.

## Procédé
1. Anciens artefacts fal rangés dans `~/dev/cent-ans-raw/dn/<id>/fal_rejected/`.
2. Images Z-Image Turbo local (mflux), 3 graines, `dn_batch.py --image-backend local --seeds 3 --until select --charter warn`.
3. Choix : meilleure graine conforme strict ; sinon meilleure en `warn` (scores notés dans production.md).
4. 3D : TRELLIS HF si quota, sinon SF3D local ; classes orientées : multi-vue si voie gratuite.
5. `cent-ans dn-ingest`, manifeste, galerie.

## État
- [ ] squelette
- [ ] images locales
- [ ] choix + 3D
- [ ] ingest
- [ ] fusion
