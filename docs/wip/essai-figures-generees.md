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

### Résultat essai A (30/10, fait)
- Image : Z-Image local, 2 graines (1337, 1338), pose A tenue, mains ouvertes vides, épée au fourreau à la ceinture.
  Gardée : 1337. Surcot lis/léopards lisible mais plutôt parti que écartelé ; couronne et mailles bonnes.
- 3D : TRELLIS HF **échoué** (quota ZeroGPU épuisé, 20 h d'attente) → **SF3D local**, 12 784 tri (12 864 après rig), tourné de 180° pour faire face à +Z.
- Rig + idle : 8 crédits (5 + 3), solde 25 → 17. Le MCP `meshy_rig` n'accepte qu'une URL publique ou un data URI
  (un glb de 650 Ko est trop gros à passer en paramètre) : appel REST direct `POST /openapi/v1/rigging` avec data URI,
  même clé que `tools/experiments/meshy_waves.py`. Tâches : rig `01a18db9-ea86-7437-b166-c6d0d345126b`, idle `01a18dba-442f-763a-b2fe-c5464d49be59`.
- 24 os, 1 action `Idle` (96 images). Rendu de contrôle Blender non vide (non relu).
- Fichiers (hors dépôt) `~/dev/cent-ans-raw/gf/menu/` : `edward_s1337.png`, `edward_s1338.png`, `edward_cut.png`,
  `edward_raw_sf3d.glb`, `edward_raw_facez.glb`, `edward_rigged.glb`, `edward_idle.glb`, `render.png`, `prompts.txt`.

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
