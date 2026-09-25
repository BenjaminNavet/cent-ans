# Lot EP5 — Porte-étendards (chantier « batailles épiques »)

Branche `worktree-agent-ae217ad35aaf7a266`. ADR : `docs/decisions/0034-porte-etendards.md`.
Plan d'ensemble : `docs/wip/epic.md`.

## État
| Partie | État |
|---|---|
| 0. Squelette, contrat | fait |
| 1. Figurines Blender (porte-étendard à pied / à cheval, tambour, busine) + clips | fait (sous-agent, commits 64bccfdd, 81b87442) |
| 2. Données (`data/rules/battle_standards.json`, `data/fx/battle_standards.json`) + schémas + pytest | fait |
| 3. Règles cœur (chute, relève, prise, trophées) + tests `sim-battle/tests/ep5_standards.rs` | fait |
| 4. Rendu Godot (`battle_standards.gd` réécrit, `battle_standard_flag.gdshader`) | fait, vérifié en bataille |
| 5. Musiciens (sons via `BattleAudio.play_at`, API existante : « drum », « horn ») | écrit |
| 6. Écran de fin (mentions + butin) + chronique (`GameEvent` Battle) | écrit, test campagne `ep5_trophies.rs` |
| 7. Captures `docs/img/ep5/` (`--standard-shot=foot|mounted|line|fallen|captured`), ADR 0034 | fait |

## Contrat figurines (Blender → Godot)
Figurines du manifeste `game/assets/models/battle_skinned/manifest.json` (nom `kind_variant`) :
`standard_0` (à pied, rig human), `standard_1` (à cheval, rig cavalry), `musician_0` (tambour),
`musician_1` (busine). `pole_top` / `pole_axis` (espace de repos ; la hampe est horizontale au
repos, toujours appliquer la matrice de `Prop` / `R:Prop`). Clips `std_*`, `drum_*`, `horn_*`,
`c_std_*`.

## Choix
- Règles : flux aléatoire propre (`BattleRng::derive`), les autres tirages des batailles ne
  bougent pas. Chute : sous 60 % de l'effectif, 3 % par point d'effectif perdu dans le pas ;
  débandade au contact : 35 %. (Réglage final : sous 50 %, 1,5 % par point ; voir ADR § Équilibre.) Tombé : −6 moral d'un coup, −0,5/s, coups ×0,85 ; ennemis à
  60 m +4. Relevé après 5 s si le régiment tient, pris si un ennemi apte est à 15 m. Pris :
  −10 moral au régiment, −5 aux voisins, plafond −10 ; preneur +8. Général ×2.
- Deux porte-étendards à partir de 120 soldats (le second porte un pennon).
- Rendu : aucun nœud par drapeau ; MultiMesh par (camp, rôle, LOD), étoffes en Texture2DArray,
  drapeau lu sur l'os `Prop` de la texture d'os ; agrandi ×3 entre 140 et 700 m, rien au-delà de 1 000 m.
- Piège : `PackedFloat32Array` passé par référence en Godot 4 : `_hide_reserved` copie le tampon
  (sinon `_previous` perd les vraies places et les porte-étendards sont écrasés).

## Prochaine étape
Smoke, fusion de main, rapport.
