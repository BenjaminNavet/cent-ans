# vq-rivers : rivières et lacs réalistes (carte de campagne)

État : fait (17/10). Bloc `look` de `data/map/river_display.json` (schéma à jour).

## Fait
- `river.wesl` (rubans moyens + lacs) : dégradé de profondeur plus doux, limon clair des bas-fonds (`mud_*`), reflet du ciel par fresnel seul (`base_reflect` 0,04 au lieu de 0,12), reflet du soleil (Blinn-Phong sur la normale d'ondulation, première lumière directionnelle), étalonnage saturation/luminosité (`saturation`, `brightness`), teinte lointaine gris-bleu `far_color` entre `far_blend` (90 -> 600 unités carte), berge humide (bord extérieur du ruban assombri, `bank_wet_start`, `bank_ink_*`).
- Couleurs de base plus sombres (vert-gris / bleu profond) ; couleurs des lacs dans `look.lake_*`.
- Lisibilité lointaine inchangée (largeurs, plancher px, alpha non touchés) : Seine visible à d=400.
- Interrupteur de banc `CENT_ANS_RIVER_LOOK_OFF=1` (look neutre : pas de grade, glint, limon, fondu lointain, berge).

## Mesures (banc carte, d=60, release dev, 3 paires ABABAB, machine très chargée)
Delta B-A : p50 -1,8 ms, p95 -6,9 ms (bruit ; deux autres séries ±, aucun surcoût systématique). Coût ALU négligeable.

## Reste
- Bande de sol mouillé hors du ruban (géométrie / décalque) non faite : seul le bord intérieur du ruban fonce.
- Rubans fins de près (`river_fine.wesl`, déjà bancs/roseaux) non retouchés.
- Étalonnage lu de data (`look`), pas du grade du terrain par saison (aucun uniforme partagé).
- Pas de fog/perspective aérienne appliquée à l'eau (la teinte lointaine la remplace).
