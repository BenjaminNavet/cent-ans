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
- [ ] V1 audit
