# PO3 — Carte de campagne (lumière, forêts, étiquettes)

Branche `feat/po3-map` (depuis `feat/po-polish`). Plan : `docs/superpowers/plans/2026-09-27-po-polish.md` § PO3.
Références : bible DA § 12.6, ADR 0097.

## État
- [x] Squelette : `game/tests/po3_shot.gd`, ce fichier.
- [ ] 1. Lumière : soleil rasant par saison dans `atmosphere.json` (`campaign.seasons.<s>`), brume bleu-or, étalonnage en S.
- [ ] 2. Forêts : teinte 3 tons + échelle ±25 % (shader), lisières basses et clairsemées (masque de couverture).
- [ ] 3. Étiquettes : encre + halo léger, taille/graisse par rang (20/17/14 px, PO2 : `UiType`).
- [ ] 4. `po_grade_test.gd` partie campagne ; `smoke.gd` ; `da7d_overlap_test.gd` ; banc PB1.
- [ ] 5. Captures `po3_shot.gd` → `docs/img/po/po3/` ; saturation DA7b ≤ 35 %.

## Banc PB1 (M4 Pro, même machine, charge ≈ 28)
- Avant : à mesurer.

## Prochaine étape
Lumière (étape 1).
