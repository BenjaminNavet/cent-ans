# 0236 — Fabrique de textures régionales (TX)

Statut : accepté (10-09, chantier TX ; amendé le même jour : fal par défaut)

## Contexte
Les textures du jeu (sols Poly Haven, parcellaire HB fal 1024, cartes de végétation 512, matières
de bâtiments Poly Haven) sont peu nombreuses, communes à toute la carte et floues de près. Le joueur
veut des textures propres à chaque coin de la carte (désert, steppe, Europe centrale, Nord…) pour
le sol, la végétation et les bâtiments, en plus haute résolution. Spécification :
`docs/superpowers/specs/2026-10-09-textures-regionales-design.md`.

## Décision
- Une **fabrique** unique, `tools/cent_ans_tools/texture_factory/` (généralisation de
  `ground_materials.py`, qui en réexporte les fonctions) : `catalog`, `generate`, `seamless`,
  `upscale`, `pbr` (normale OpenGL + rugosité, micro-détail), `alpha` (rembg local), `checks`,
  `pack`, `board`. Commande `cent-ans textures <étape> <famille>`, chaque étape reprenable depuis le
  disque ; cache brut hors dépôt `~/dev/cent-ans-raw/textures/<famille>/`.
- **Catalogues** de données `data/art/textures/<famille>.yaml`, validés par
  `data/schemas/texture_catalog.schema.json` ; les prompts ne vivent jamais dans le code.
- **Générateur** : Z-Image Turbo sur fal.ai (`fal-ai/z-image/turbo`, 0,005 $/Mpx) en **2048 natif**,
  8 appels en parallèle, enveloppe 10 $ validée par le joueur (`docs/budget.md` § TX) ; la commande
  affiche le coût estimé et refuse au-delà de `--max-cost`. Route locale de secours (`backend:
  local`) : mflux (ADR 0190) en 1024/1536 puis Lanczos vers 2048 (< 1 s) ; Real-ESRGAN seulement
  ponctuel (143 s par image sur M4 Pro). Un échec fal n'est pas refait en local dans le même lot.
- **Sélection** : une graine par matière ; contrôle raté → graine + 1, deux fois au plus, puis
  `flagged` sur la planche ; on ne refait que les ratés flagrants.
- 2k + texture de micro-détail par famille fondue de près ; pas de 4k.

## Conséquences
- ~330 images pour ≈ 7 $ et moins d'une heure de génération, au lieu de ~6-17 h en local.
- Les textures Poly Haven restent derrière `--legacy-textures` jusqu'à la partie pilote.
- Les biomes passent à 14 (ADR 0237) pour porter les variantes régionales.
