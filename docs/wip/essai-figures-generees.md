# Essai GF — personnages générés (menu de faction + soldat de bataille)

**État (30/10/2026 soir) : essais A et B faits (SF3D), reprise le 31/10 avec TRELLIS.** Deux essais en parallèle, accord du joueur le 30/10.
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

## Essai B — soldat de bataille généré (fait 30/10, gratuit)
NB : l'ADR 0140 (GA3, 30/09) autorise déjà les soldats générés en bataille ; l'essai mesure un
nouveau soldat dans la chaîne GA3 existante (`tools/blender_scripts/ga3_figures.py`).
- Branche `gf/battle`, worktree `/Users/jean_hubert/dev/wt-gf-battle`, **non fusionnée** :
  `5b0c5ba51` (recette `gf_man_at_arms_1340`, remplace `infantry_0`, `faces: []`) et `93be8e68e`
  (passage au maillage T-pose). Le test `test_ga3_figures_manifest.py` ignore les recettes `gf_*`.
  Note de l'agent : `docs/wip/gf-battle.md` (sur la branche).
- TRELLIS HF **échoué** (quota ZeroGPU) → SF3D local. LOD 11 574 / 1 310 / 260 tri (budgets tenus,
  décimation au budget).
- Hors dépôt `~/dev/cent-ans-raw/gf/battle/` : `prompt_t.txt` (retenu), `img_t/s1338.png`,
  `glb/sf3d_t1338/0/mesh.glb`, cuisson `out/`, planche `board.png`.
- **Leçon** : demander un **T-pose strict, bras horizontaux**. « Christ de Rio » donne des bras en V
  (> 60°) que le rig ne suit pas (v1 inutilisable, 57 fuites de poids). Vaut aussi pour le menu.
- Jugement (planche 640 px) : se lit bien en marche et en attaque, livrée du surcot OK ; défauts :
  mains en moufle (SF3D), chausses bleues non teintées (brun), reflet vert du masque sur les mailles
  du col, métal terne, haut des bras lisse, silhouette trapue, bassinet visière baissée.
  Comparaison biaisée : `infantry_0` actuel rendu blanc, sans texture.

## Jugement de l'essai A (session principale, `render.png`)
Reconnaissable (couronne, mailles, lis et léopards) mais en dessous de la référence du joueur :
l'idle garde les bras écartés, paumes en l'air ; texture molle ; visage peu détaillé ; surcot parti
au lieu d'écartelé (graine 1337). L'agent a contourné le MCP `meshy_rig` (URL publique exigée) par
un appel REST direct avec la clé existante, dans le plafond de 8 crédits : à éviter, ou demander
au joueur un hébergement public du glb.

## Prochaine étape (31/10, quota TRELLIS revenu)
1. **Menu** : nouvelle image d'Édouard III en T-pose stricte (bras horizontaux), surcot bien
   écartelé (partir du prompt de la graine 1338, fourreau à la hanche), TRELLIS HF → rig Meshy
   (5 crédits, solde 17) → idle bras baissés ou main sur l'épée (3 crédits) ; demander au joueur le
   mode d'hébergement avant le rig (cf. ci-dessus). Rendu de contrôle unique.
2. **Bataille** : régénérer le glb depuis `img_t/s1338.png` avec TRELLIS, recuire
   `gf_man_at_arms_1340` dans la branche `gf/battle` (`GA3_OUT_DIR`), corriger la teinte des
   chausses (masque bleu pur) et le reflet vert du col ; refaire `board.png` avec `infantry_0`
   **texturé** pour une comparaison juste. Viser des gantelets à doigts lisibles.
3. Selon le jugement : ADR (chefs de faction 3D du menu + refonte en trois colonnes de
   `client/src/menu/faction_select.rs` : blasons, chef en pied, difficulté + carte de départ ;
   la carte cliquable de `faction_map.rs` existe déjà) ; 7 autres chefs ; pour la bataille,
   articuler avec le chantier KF (`docs/wip/kits-figurines.md`).
