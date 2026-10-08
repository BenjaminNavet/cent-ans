# ADR 0210 — Procédé gratuit des assets 3D : Qwen / Z-Image → TRELLIS (HF) → SF3D

Statut : accepté (08/10, choix du joueur après les essais I3D 1-4).

## Contexte

fal.ai n'a plus de crédit (ADR 0152) et les essais du 08/10 (`docs/wip/i3d-local.md`) ont comparé
les chaînes gratuites image → 3D. TRELLIS reste le meilleur et tourne gratuitement sur le Space HF
`trellis-community/TRELLIS` (quota ZeroGPU quotidien) ; Stable Fast 3D local est bon depuis une vue
de face ou ¾ avant ; TripoSR est abandonné. Côté image, Z-Image Turbo (ADR 0190) ne lit que le
texte, alors que Qwen-Image-Edit-2511 (Apache 2.0, local) accepte des images de référence et
retouche une pose sans rien changer d'autre, mais demande 25-35 min par image sur la machine.

## Décision

- Procédé de référence : `docs/pipeline-assets-3d.md`.
- Figurines : image par Qwen-Image-Edit-2511 avec une figurine validée en référence ; décor,
  accessoires, végétation : Z-Image Turbo. La charte est vérifiée sur l'image avant toute 3D.
- 3D : TRELLIS gratuit (Space HF, `tools/experiments/trellis_hf.py`) en premier, SF3D local en
  second, toujours nourri d'une vue ¾ avant.
- Meilleur-de-N (plusieurs images, plusieurs générations 3D, on garde le meilleur) pour les modèles
  gratuits seulement ; un seul appel pour un modèle payant (fal, Nano Banana 2), qui reste le
  dernier recours.
- Figurines : bras levés presque à l'horizontale (« Christ de Rio »), paumes ouvertes et vides.

## Conséquences

- Coût nul par défaut ; le débit est limité par le quota ZeroGPU (1-3 générations TRELLIS par jour
  en compte gratuit, chaque appel réservant 120 s) et par la lenteur de Qwen en local.
- HF PRO (25 min/jour) lèverait la limite de TRELLIS : dépense à décider par le joueur.
- Le test des vues générées par le LoRA d'angles de Qwen n'a pas été mené à terme (trop long) ;
  à éprouver au premier asset qui en a besoin.
