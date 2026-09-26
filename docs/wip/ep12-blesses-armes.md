# Lot EP12 — Blessés qui rampent, fuyards qui jettent leurs armes

Branche `worktree-agent-ab10bccc115923724`. Rendu seulement (game/, tools/blender_scripts, data/fx).
ADR 0072 (0070-0071 laissées à EP11/EP13 qui tournent en parallèle ; renuméroter à la fusion si besoin).

## Plan
1. Kit Blender (`battle_skinned_poses.py`, `battle_skinned.py`) : clips `crawl`, `wounded_sit`,
   `wounded_kneel` (non bouclés, finissent immobiles), `flee` / `flee_m` (course de fuite sans arme).
   Drapeau de face `HELD_MASK = 64` sur les armes tenues et boucliers (masqué par le shader quand le
   régiment fuit). Régénération : `blender --background --python tools/blender_scripts/battle_skinned.py
   -- --only infantry_0,...,archer_5`.
2. Shader : `drop_arms` (uniforme) et code 7 des cadavres (« désarmé ») masquent les faces `HELD_MASK`.
3. `battle_soldiers.gd` : part des morts tirée blessée (hachage id régiment × rang de la perte),
   couche de cellules « blessés » (jeu de clips blessés), plafonds ; déroute → `drop_arms`, clips de
   fuite, objets au sol (`battle_dropped_arms.gd`, MultiMesh plafonnés).
4. `data/fx/battle_gore.json` sections `wounded` et `dropped_arms` + schéma.
5. Banc 15 000 avant/après, captures `docs/img/ep12/`.

## État
- Squelette (ce fichier).

## Prochaine étape
Clips Blender.
