# GA3-L4 — Figurines générées : défauts + recettes restantes

Branche `feat/ga3-l4` (worktree agent). Budget L4 ≤ 3 $ (cumul GA3 de départ 6,55 $).
Brutes : `~/dev/cent-ans-raw/ga3/l4/`. Contexte : `docs/wip/ga3.md` (L3a-c), `docs/wip/ga3-l3.md`.

## Consigne de reprise
> Lis ce fichier et `git log --oneline -15`, continue à la première case non cochée.

## Décisions
- **Variantes de visage = têtes greffées** (pas de corps dupliqué : coût sommets) : la
  planche NB2 de la variante est un *edit* de la planche A-pose déjà générée (`--variant k`,
  1K) qui ne change que la tête ; corps partagé ; la tête A et la tête B deviennent des pièces
  à masque de variante (bits 0, 1…), le corps garde un bouchon au cou. Hauteur du casque de B
  = celle de A × rapport des hauteurs de figure sur les deux planches découpées. Tête B cuite
  dans une bande 512 px sous l'atlas (albédo 1024 × 1536). LOD0 et LOD1 seulement (LOD2 : tête A
  pour tous).
- **Mains** : greffe au niveau `CAM` des mains fermées de la figurine fine exportée (même rig,
  poing fermé en pose de liaison, gants/gantelets de la recette), moufle générée retirée
  (triangles du corps entièrement sur `Wrist`). LOD0 seulement.
- **Usure SR2** dans `GA3_TEX` (boue par hauteur, crasse par soldat, acier patiné `C_PLATE`),
  teint par soldat et rougeur réduite sur les texels de peau au-dessus du cou (`ga3_head_y`).
- Coût : 1K partout en L4 (0,08 $ + bria 0,018 + multi 0,02 = 0,118 $ par génération).

## Étapes
- [ ] 1. Squelette (note, options fal `--variant`, entrées UNITS) — commit
- [ ] 2. Shader : usure SR2 + peau dans `GA3_TEX`, `sr2_weathering_test.gd` adapté
- [ ] 3. Chaîne : têtes variantes (fal + Blender), mains fines greffées, manifeste `variants`
- [ ] 4. Variantes B des 6 recettes existantes (génération + build)
- [ ] 5. 10 recettes restantes (A puis B selon budget)
- [ ] 6. Tests (ga3_l3 deux modes, an1a, an1b, sr2, fg3_maps, nt7, nt10, smoke, pytest), planche
- [ ] 7. Docs (ga3.md § L4, ADR 0140, budget), merge main

## Journal
