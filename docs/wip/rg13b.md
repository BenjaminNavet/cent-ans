# RG13b — figures et accessoires 3D de la salle du conseil

Spec : ADR 0341 § Décision 4 ; pipeline : `docs/pipeline-assets-3d.md` (ADR 0210, gratuit seulement).
Direction artistique (consigne du 11/10) : réalisme sombre et patiné, palette désaturée (sang de bœuf,
terre d'ombre, charbon, vert bouteille, or terni), cuir, laine épaisse, brocart, fourrure ; aucun nom
ni insigne d'une œuvre existante dans les prompts.

Prompts : `data/art/council_catalog.json`. Sorties brutes hors dépôt : `~/dev/cent-ans-raw/council/`.
Cibles : `game/assets/models/council/council_{ruler,chancellor,constable,treasurer,chaplain,chamberlain,table,throne,stool}.glb`.

## État
- [x] catalogue de prompts
- [ ] images Z-Image (3 graines x 9 objets, `gen.py`)
- [ ] détourage rembg, choix, 3D (TRELLIS HF puis SF3D), ingestion, glb

## Échecs / quota
(aucun pour l'instant)
