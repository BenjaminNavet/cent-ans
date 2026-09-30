# GA3 — Sondes image-vers-3D (fal.ai TRELLIS)

Branche `feat/ga3`, worktree `../game_project-ga3`. Clé `FAL_KEY` dans l'environnement (vérifiée 30/09).
Plan d'origine : `docs/wip/ga.md` §GA3. Budget GA3 ≤ 8 $ (sondes < 1 $). Dépenses → `docs/budget.md`.
Ne pas toucher `feat/sr` (autre session, figurines semi-réalistes).
Brutes (hors dépôt) : `~/dev/cent-ans-raw/ga3/` ; références figurines : `~/dev/cent-ans-raw/sr3/*.png` (1K, face/profil/dos).

## Consigne de reprise
> Lis ce fichier et `git log --oneline feat/ga3 -15`, continue à la première case non cochée.

## Sondes (go/no-go du joueur à la fin)
- [x] S1 décor : maison à colombages → TRELLIS → `tools/blender_scripts/ga3_cleanup.py`
      (décimation, LOD0/1/2, cuisson albédo 1024²) → `game/assets/models/props_ga/` ; planche
      `docs/img/ga3/s1_house.jpg`.
- [ ] S2 figurine : longbowman (`sr3/longbowman.png`, vues multiples) → TRELLIS → nettoyage →
      rattachement au squelette de bataille existant (poids automatiques) → rendu marche + tir ;
      planche `docs/img/ga3/s2_archer.jpg` ; verdict technique (déformations, triangles, coût).

## Journal
- 30/09 : worktree créé, clé validée, joueur OK pour les 2 sondes.
- 30/09 : **S1 fait** (≈ 10 min de bout en bout, 0,05 $). Image `fal-ai/flux/dev` 1024² (0,026 $,
  6 s ; brutes `~/dev/cent-ans-raw/ga3/s1/`). Prompt : « Photorealistic photograph of a small
  14th-century French peasant half-timbered house, exposed dark oak timber framing with lime-washed
  wattle and daub infill, steep thatched roof, … Three-quarter view from slightly above, … isolated on a
  plain uniform neutral mid-grey studio background, no ground, … soft diffuse overcast lighting » →
  `fal-ai/trellis` (0,02 $, 25 s ; `texture_size` 1024, `mesh_simplify` 0,95, seed 1337) : 25 409 tri,
  UV + texture 1024² déjà fournis → `ga3_cleanup.py … ga3_house --strip-base 0.35` (Blender, < 1 min) :
  LOD0 7 998 / LOD1 4 000 / LOD2 1 200 tri, 8,00 × 6,77 × 7,71 m (h), pied à y = 0, albédo recuit
  1024/512/256 (Cycles, sélection vers active). glb 953/454/137 Ko. Contrôle `ga3_s1_shot.gd`
  (headless sans `--out`) OK. Planche `docs/img/ga3/s1_house.jpg`.
  Défauts : (1) TRELLIS reconstruit le socle d'herbe de l'image → `--strip-base` le retire, restent
  quelques touffes collées au pied des murs ; demander « no ground » ne suffit pas à flux, prévoir un
  détourage (fond retiré) avant TRELLIS pour le lot. (2) Faces cachées inventées (porte absente du
  côté arrière, pignon arrière plus pauvre) : acceptable vu de loin en bataille. (3) Albédo un peu
  sombre et terne (texture TRELLIS + ombres cuites de l'image) ; pas de normal map. (4) Anachronismes
  venus de l'image (cheminée de briques à mitre rouge, deux niveaux) : à régler au prompt.
  (5) Godot extrait les textures embarquées à côté des glb (`*_albedo.jpg`, commités avec les .import).
  **Avis : go pour le lot décor** (qualité très au-dessus du kit procédural pour ≈ 0,05 $/objet) avec
  deux ajouts au pipeline : détourage de l'image source et correction d'exposition de l'albédo ; les
  maisons de bataille sont vues de loin, le LOD0 8 k est confortable, LOD2 1,2 k suffit au-delà de 80 m.
  Point ouvert : `game/bin/` sans dylib dans ce worktree (sans effet sur la sonde, `core/build.sh` si besoin).
