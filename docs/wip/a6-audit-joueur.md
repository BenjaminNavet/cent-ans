# A6 — Audit joueur : corrections

Tableau : `docs/audit/a6-audit-joueur.md`. Agents `cent-ans-mech` (Sonnet), un worktree chacun ; relecture puis fusion ff-only par la session principale.

## Lots
| Lot | Constats | État |
|---|---|---|
| L1 prévision = résolution auto | M1 M2 | vague 1 |
| L2 sièges (assaut sans brèche, engins) | M3 | vague 1 |
| L3 économie, rançons, capture du souverain | M4 M5 | vague 2 (après la sonde A2) |
| L4 recherche (points non perdus, file) | M7 U5 | vague 1 |
| L5 diplomatie (règle déterministe, débordement, bouton commerce) | M6 U15 U16 | vague 1 |
| L6 fenêtres et notifications (filtre de pertinence, file modale, pile de lettres) | U6 U7 U8 U10 U17 U20 | vague 1 |
| L7 colonie (listes recrutement/construction, défilement, file de construction) | M8 U11-U14 | vague 1 |
| L8 sélection et barre du haut (armée en ville, écu, infobulles, ellipse) | M9 U4 U9 U19 U21 | vague 1 |
| L9 lisibilité de la carte (remplissage, étiquettes, palettes, météo, religion) | C1-C9 U18 | vague 1 |
| L10 performance (appels de dessin au zoom max, ultra, fuites) | P2-P4 | vague 2 |
| L11 menus d'accueil (sous-menu batailles, choix de faction, citations) | U1-U3 | vague 1 |
| L12 interface et caméra de bataille, sol | U22 U23 B2-B4 | vague 1 |
| L13 durée des batailles | B1 | vague 2 |

## Prochaine étape
Attendre les rapports de la vague 1, relire chaque branche, fusionner ; lancer L3, L10, L13.

## L9 (lisibilité de la carte) — fait, branche worktree-agent-aebbedcd06cbb5c34
C1 stance_fill (alpha self 0,45→0,62, enemy 0,42→0,52, friend 0,35→0,45, saturation 0,8→1,0, flat_mix 0→0,35) ; C2 rivières proches seulement + `major_names` (0,22 de l'étendue) ; régions alpha 0,5→0,85 ; C3 `plate_scale` 0,8 et écus rang 1/2 160/380 ; C4/C5 grade été/hiver + bloc `ground` de campaign_seasons.json ; C6 `rain_amount_scale` 0,45 et amount_ratio_far 0,45 ; C7 `parchment_icon_cell_px` 300 ; C8 couleurs en constantes de map_mode_controller (islam vert, Avignon bleu, hérésie violet) ; C9 `cumulus_medium_scale` 0,3 ; U18 `hover_bar`. Reste : jugement visuel (`a6_map_shots.gd`) par la session principale.
