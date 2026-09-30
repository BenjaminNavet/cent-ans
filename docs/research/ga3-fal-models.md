# GA3 — modèles fal.ai image-vers-3D (relevé 30/09/2026)

Source : API `api.fal.ai/v1/models` (catégories image-to-3d, 3d-to-3d) + pages `fal.ai/models/<id>/llms.txt`.
Prix par modèle généré (attention : l'API de tarifs donne des « credits »/« units », pas le prix final).

| Modèle | Date | Prix / modèle | Entrée | Remarques |
|---|---|---|---|---|
| `fal-ai/trellis` (+ `/multi`) | 2024-12 | 0,02 $ | 1 ou n images | S1 ; ancien mais imbattable en prix |
| `fal-ai/trellis-2` | 2025-12 | 0,25 / 0,30 / 0,35 $ (512/1024/1536) | image | successeur, géométrie plus fine, PBR |
| `tripo3d/h3.1/image-to-3d`, `/multiview-to-3d` | 2026-04 | 0,20 (sans tex) / 0,30 / 0,40 $ HD ; +0,20 géom. détaillée, +0,05 quads | 1 ou 4 vues | multi-vues natif |
| `fal-ai/hunyuan-3d/v3.1/rapid` | 2026-01 | 0,225 $ (+0,15 PBR) | image | |
| `fal-ai/hunyuan-3d/v3.1/pro` | 2026-01 | 0,375 $ (+0,15 PBR, +0,15 multi-vues, +0,15 nb faces imposé) | 1 ou n vues | nb de faces cible |
| `fal-ai/hunyuan3d-v3` LowPoly | 2025-12 | 0,45 $ | image | sortie basse poly directe |
| `meshy/v7.1/multi-image-to-3d` | 2026-09 (dernier) | 0,80 sans tex / 1,20 $ texturé ; +0,20 rig auto, +0,12 anim | n vues | seul avec **rig + anim** intégrés (1,52 $ tout compris) |
| `tripo3d/p2/image-to-3d` | 2026-09 | 1,00-1,30 $ | image | haut de gamme |
| `hitem3d/hi3d/v3.0` | 2026-08 | 2,10 / 9,10 $ | 1 ou n vues | très cher |
| `fal-ai/hyper3d/rodin/v2.5` | 2026-05 | 0,40 $ (fast 0,10 $) | image | |

Outils annexes : `fal-ai/meshy/rigging` 0,20 $ (+0,12 anim) sur un maillage existant (ex. sortie TRELLIS) ;
`tripo3d/tripo/remesh`, `tripo3d/tripo/segment` (séparer arc/corps) ; `fal-ai/hunyuan-3d/v3.1/part` ;
`fal-ai/bria/background/remove` 0,018 $.

Images sources : `fal-ai/flux/schnell` 0,003 $/Mpx, `fal-ai/flux-2` 0,012 $/Mpx, `fal-ai/flux/dev` 0,025 $/Mpx,
`nano-banana-2/edit` 0,08 $ (bon pour planches multi-vues cohérentes). Récents : seedream v5, gpt-image-2.5, qwen-image-3.

## Recommandation
- **Décor** : image `flux-2` (0,012 $) → détourage → `trellis` 0,02 $ ; pour les bâtiments-clés, `trellis-2` 1024 (0,30 $).
  Lot de ~10 objets : 0,3 $ (trellis) à 3 $ (trellis-2).
- **Figurines animées** : TRELLIS (S2) si le rig sur notre squelette tient ; sinon comparer `tripo3d/h3.1/multiview`
  (0,30 $, 4 vues de nos planches) et `meshy/rigging` (0,20 $) — mais le rig Meshy n'est pas notre squelette
  (retargeting des animations à faire). ≈ 6 unités × 0,5-1,5 $ = 3-9 $.
