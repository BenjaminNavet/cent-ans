# Q4 — corrections après la recette Q3

Branche : `worktree-agent-a8324042adc365e4d` (Q3 + main fusionnés). Source : `docs/audit/q3-recette.md`.
Captures : `docs/audit/captures/q4/`. Pilote : `game/tests/q3_playtest.gd` (`user://settings.cfg`
sauvegardé puis restauré).

| # | Défaut | État | Commit |
|---|---|---|---|
| 1 | P1 bulle du conseiller | fait, vérifié en fenêtre : 3 unités recrutées à Paris au tour 1 pendant qu'il parle (`p1-advisor-recruit-paris`) ; aucune bulle sur l'avant-bataille, la fin de bataille, le rapport de saison ; la réplique attend la fin du discours (`p2-siege-rouen-start-clear`) | 43be1ab3 |
| 2 | P1 ordre des calques | fait : en-tête de `game/scripts/ui/panel_stack.gd` (calques + étages `Tier` + `restack` dans `MapUI` + groupe `ui_blocking`) | 43be1ab3 |
| 3 | P1 avant-bataille navale | cœur correct, test `core/crates/sim-campaign/tests/q4_naval_player_side.rs` (convoi français intercepté → `Defender`, escadre française → `Attacker`, `win_chance` = part du joueur). La capture Q3 `nv-006` montre bien la France à gauche (80 %) ; la défaite venait du combat 3D sans ordres (IA contre IA : 3/5 pour la France). Aucun changement de code. | 7d160027 |
| 4 | P2 siège 3D | fait : le `WorldEnvironment` de la carte restait dans l'arbre et gagnait sur celui de la bataille (brouillard de la carte sur toutes les batailles lancées depuis la campagne) ; la carte le retire pendant la bataille. Caméra d'assaut : derrière l'armée, face à la porte. `--open-shot` ajouté au banc de bataille. | 0323ae7b |
| 5 | P2 Haute ≈ Ultra | mesuré, `relief_vertex_px` 6 → 7,5 en Haute | a5540020 |

## Mesures Q4 (`--journey --map-ab=40 --uncapped`, 1920×1080, M4 Pro, autres sessions actives)
Vue de Paris à 40 u., temps d'image médian (ms), primitives :
| Config | ms | prim. |
|---|---|---|
| Haute (avant) | 25,7-25,9 | 7,81 M |
| Ultra | 37,4-38,5 | 9,93 M |
| Moyenne | 19,3-19,8 | 6,05 M |
| Basse | 9,5 | 2,35 M |
| Haute sans MSAA | 19,4 | 7,81 M |
| Haute sans ombres | 21,8 | 3,07 M |
| Haute sans végétation | 23,9 | 2,72 M |
| Haute, arbres sans ombre | 24,6 | 4,08 M |
| Haute, arbres 75 % | 24,4 | 6,64 M |
| Haute, relief 7,5 px (appliqué) | 24,2 | 7,57 M |
| Haute, relief 8 px | 23,5 | 7,54 M |
| Haute, SSAO coupé / ombres douces basses / 2 cascades | 25,6 / 25,5 / 25,2 | — |

Haute n'est plus « aussi lourde qu'Ultra » : ≈ 45 % de temps en plus en Ultra. Les chiffres Q3
(15 M primitives en Haute) venaient d'une base antérieure ; `q3_playtest` (Basse → Ultra → Haute)
mesure aujourd'hui 2,35 / 9,54 / 7,80 M.

Proposition (non appliquée, change l'aspect) : MSAA 2× → FXAA en Haute sur la carte (−6,4 ms,
bords d'arbres moins nets sur écran non Retina) ; ombres des arbres limitées à 150 u. (−1,1 ms) ;
densité d'arbres 0,85 (≈ −0,8 ms). Ensemble : Haute ≈ 17 ms (≈ 60 i/s) près de Paris.

## Points ouverts
- Discours de siège : la caméra du discours longe la ligne et cadre de près une mantelet/bélier
  (lot SG / B3, hors Q4).
- Bataille : EP fusionné, la bataille de la recette s'est finie seule en 4 min 58 s (défaite).
