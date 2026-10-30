# Essai GF — personnages générés (menu de faction + soldat de bataille)

**État (30/10/2026) : lancé.** Deux essais en parallèle, accord du joueur le 30/10.
Contexte : le joueur veut un écran de choix de faction en trois colonnes (blasons, chef de faction
en pied au centre, difficulté + carte de départ) et conteste le point 3 de l'ADR 0136 (« pas de
soldats générés en 3D »). Les deux essais mesurent si l'image → 3D tient.

## Essai A — chef de faction du menu (Édouard III, 1337)
- Image source en pose A (règle « Figure A-pose ») : couronne, surcot écartelé France/Angleterre,
  mailles, épée au fourreau (rien en main), via Z-Image local (gratuit).
- 3D : TRELLIS gratuit (HF Space) ; repli SF3D local.
- Rig + animation de repos : Meshy rig (5 crédits) + animate (3 crédits). **Plafond 8 crédits**
  (solde 25 au 30/10). Consigner dans `docs/budget.md`.
- Sortie : glb hors dépôt `~/dev/cent-ans-raw/gf/menu/`, captures de contrôle hors dépôt.

## Essai B — soldat de bataille généré
- Un homme d'armes à pied 1340 (bassinet + camail + jupon neutre), image pose A → TRELLIS.
- Réduction ≤ 2 400 tri LOD0 (+ 500 / 230), armature `human` 22 os existante posée par script
  Blender, poids automatiques ; passer marche + attaque.
- Comparer à la figurine actuelle (`docs/pipeline-assets-3d.md` § « Figurines de bataille : glb → jeu »).
- Gratuit. Aucun crédit Meshy.

## Après
- Si A tient : 7 autres chefs + refonte de `client/src/menu/faction_select.rs` (ADR).
- Si B tient : ADR qui remplace le point 3 de l'ADR 0136, articulé avec le chantier KF.

## Prochaine étape
Attendre les rapports des deux agents ; capture de contrôle unique par essai.
