# M8 — Sièges : spécification

Date : 2026-09-23. Deux volets : la guerre de siège sur la carte de campagne (fait), puis la bataille de
siège en 3D sur le moteur temps réel de M7 (à faire après la fusion de M7).

## 1. Campagne (`sim-campaign/src/siege.rs`) — fait
- `SiegeState` gagne `turns_elapsed`, `supplies` (0-100, départ 100 − dévastation/2) et `breach` (0-100).
- Vivres : perte par tour = 100 / (2 + fortifications), raccourcie par le `SiegeSpeed` du général
  assiégeant ; à 0, la garnison affamée capitule (prise de la place). `turns_left` = estimation de la
  reddition, affichée.
- Brèche : Σ `siege_attack` des engins (trébuchet 70, mangonneau 40, bombarde 85) × effectif relatif /
  (2 × (1 + fortifications)) par tour ; dès 50, ou avec une tour de siège (`wall_assault`), l'assaut ne
  subit plus la pénalité des murailles (-30 %).
- Ordre `assault { army }` : bataille auto-résolue armée contre garnison (général : gouverneur), victoire =
  prise de la place (hook `on_siege_won`, score de guerre), défaite = pertes, le siège continue.
  `CampaignState::assault_odds(army)` estime les chances pour l'interface et l'IA (assaut si ≥ 65 %).
- Sortie : la garnison attaque si sa puissance dépasse 1,3 × celle des assiégeants ; victoire = siège levé.
- Pont : `get_province_state().siege` gagne `turns_elapsed`, `supplies`, `breach` ;
  `get_assault_odds(army)`, `get_pending_events()`. Interface : ligne de siège et bouton « Donner l'assaut
  (chances ≈ x %) » dans le panneau d'armée (`scripts/map/siege_controller.gd`). Capture
  `docs/img/godot-siege.png` (`--stage=siege`). 7 tests (`tests/m8.rs`).

## 2. Bataille de siège 3D (après M7)
- `sim-battle` : carte de siège (enceinte polygonale avec tours et porte, épaisseur selon les fortifications,
  brèches existantes reportées depuis `breach`), défenseurs sur le chemin de ronde (bonus de tir et de
  défense), échelles (infanterie, lent, vulnérable), tours de siège (portent l'infanterie sur le rempart),
  bélier contre la porte, engins qui tirent sur les murs pendant la bataille, défenseurs qui abandonnent le
  rempart en déroute ; victoire de l'assaillant = contrôle de la place centrale pendant 60 s ou défenseurs
  en déroute.
- L'ordre `assault` du joueur, si `interactive_battles`, crée une bataille en attente de type siège
  (même flux que M7 : dialogue « Livrer l'assaut / Résolution automatique »).
- Godot : murailles, tours, porte low-poly procédurales (ou modèles Blender de M10), échelles et tours de
  siège animées, défenseurs positionnés sur les remparts.
- Tests : une brèche large facilite l'assaut, les échelles sans brèche sont coûteuses, le bélier ouvre la porte.
