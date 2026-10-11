# RG13b — figures et accessoires 3D de la salle du conseil

Spec : ADR 0341 § Décision 4 ; pipeline : `docs/pipeline-assets-3d.md` (ADR 0210, gratuit seulement).
Direction artistique (consigne du 11/10) : réalisme sombre et patiné, palette désaturée (sang de bœuf,
terre d'ombre, charbon, vert bouteille, or terni), cuir, laine épaisse, brocart, fourrure ; aucun nom
ni insigne d'une œuvre existante dans les prompts.

Prompts : `data/art/council_catalog.json`. Sorties brutes hors dépôt : `~/dev/cent-ans-raw/council/`.
Cibles : `game/assets/models/council/council_{ruler,chancellor,constable,treasurer,chaplain,chamberlain,table,throne,stool}.glb`.

## État : TERMINÉ
- [x] catalogue de prompts (`data/art/council_catalog.json`, avec graine retenue, modèle 3D, lacet)
- [x] images Z-Image local, 3 graines x 9 objets, meilleure retenue à l'oeil
- [x] 6 figures : TRELLIS HF ; table, trône, tabouret : SF3D local (quota ZeroGPU épuisé après 6 appels)
- [x] ingestion `dn-ingest --lods 1` (LOD0 seul), glb dans `game/assets/models/council/`

## Notes pour la scène
- Chaque figure a son tabouret/banc intégré (le modèle image l'a dessiné) : ne pas ajouter de tabouret sous elles ;
  le trône du souverain est séparé : poser la figure du souverain devant/à côté ou masquer son banc dans l'assise.
- Origine au sol au centre de l'emprise, regard -Z, mètres. Figure assise 1,30 m. Table 3,2 x 1,1 x 0,85 m
  (déformée non uniformément, SF3D la rendait trop trapue). Trône 1,6 m de haut. Tabouret 0,45 m.
- Textures désaturées d'origine ; pas de masque vert/bleu de livrée (pas de blason sur ces figures).

## Échecs / quota
- TRELLIS HF : quota ZeroGPU épuisé après 6 générations (souverain + 5 figures) ; table/trône/tabouret en SF3D.
- SF3D du tabouret depuis la graine 1338 : texture noire ; refait depuis la graine 1337 (correct).
- Constable : TRELLIS l'a orienté dos à -Z, ingéré avec lacet 0 (les autres 180).
