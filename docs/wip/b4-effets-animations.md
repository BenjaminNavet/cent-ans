# Lot B4 — Effets et animations de bataille (fusionné dans main, `2a485a5`)

Plan : `docs/design/2026-09-24-rapprochement-total-war.md` (lot B4). Suite de B1 (`docs/wip/b1-maillages.md`).

## État : fusionné (24/09), banc d'essai fait
| Partie | État |
|---|---|
| Rythme d'engagement (IA `sim-battle`) | **fait** : contact de la démo à 70 s (au lieu de 301 s), test |
| Animations (shader + sommets) | **fait** : archer, arbalétrier, cavalier, caparaçon, mêlée, chutes |
| Effets (`battle_effects.gd`) | **fait** : poussière, éclaboussures, traits, bombardes, choc des charges |
| Captures `docs/img/b4/` | **fait** (avant/après), sauf bombarde en bataille réelle (pas de bombarde dans la démo) |
| Banc d'essai `--benchmark` A/B | **fait** (voir « Mesures ») : coût des effets non mesurable en FPS |
| ADR | pas commencé (facultatif : aucun choix d'architecture nouveau hors ADR 0006) |

## Tests
- `cd core && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test` : vert
  (au commit 7eacc6c ; le Rust n'a plus changé depuis).
- Smoke `godot --headless --path game --script res://tests/smoke.gd` : vert (code de retour 0, aucune
  `SCRIPT ERROR`) au commit d864eaf ; depuis, seuls des PNG et ce fichier ont changé.
- La dylib doit être reconstruite (`core/build.sh --release`, puis copie en `libcent_ans.debug.dylib`
  si l'éditeur cherche la version debug) et `godot --headless --path game --import` relancé une fois
  (nouvelle classe globale `BattleEffects`, sinon « Could not find type BattleEffects »).

## Fichiers touchés
- Rust : `core/crates/sim-battle/src/ai.rs` (duel perdu → contact, assaut de la cavalerie,
  `ATTACKER_WAIT` 90 s), `tests/f5d.rs` (contact < 180 s), `tests/fixtures/demo_battle_1337.json`
  (statistiques actuelles : armures +5, arcs longs 78/73, moral).
- Blender : `tools/blender/battle_figures.py` (+ tous les `.glb` et `figures.json` régénérés) :
  poids de coude (pivot = « knee » des membres 3/4), membres 18 (jambes du cavalier), 19 (rênes,
  poids = part du poing gauche, pivot « knee » = poing), 20 (flèche encochée), corde à 3 points
  (poids = part de l'allonge), écu pondéré 1 (suit l'avant-bras).
- Shaders : `battle_soldier.gdshader` (réécrit : chaîne coude → épaule (flexion, abduction) →
  torsion → inclinaison ; uniformes `elbow`, `volley_time`, `reload_time`, `state_time`),
  `battle_projectile.gdshader`, `battle_projectile_trail.gdshader`, `battle_projectile.gdshaderinc`.
- Scripts : `game/scripts/battle/battle_effects.gd` (nouveau), `battle_soldiers.gd` (uniformes,
  `soldier_positions()`), `battle_scene.gd` (effets, `--no-effects`, `--shot-at=<s>`).
- Tests/outils : `game/tests/b4_figures_shot.gd` (gros plans : archers, crossbows, charge, melee,
  bombard, volley ; `--time`, `--since`, `--cam`), `game/tests/b4_pacing.gd` (rythme sans rendu).

## Rythme (point 3)
Cause : pas l'attente d'un attaquant plus faible (rapport de forces 1,25 au départ), mais le **duel
d'archerie** : les arbalétriers français « tenaient » leur ligne tant que `own_ranged >= 0,8 ×
enemy_ranged` (puissance théorique) pendant `DUEL_TIME` = 480 s, alors que les arcs longs anglais
(renforcés depuis, cf. écart de la fixture) gagnaient nettement l'échange ; la ligne n'avançait
qu'une fois les carreaux épuisés (≈ 300 s). Corrections (`ai.rs`) :
- un camp qui perd l'échange (pertes > pertes ennemies + `DUEL_LOSS_MARGIN` 4 %) cesse le duel ;
- dans ce cas l'attaquant lance aussi sa cavalerie sur la cavalerie ennemie (règle « assaut » F5d) ;
- un attaquant plus faible n'attend plus le défenseur que 90 s (`ATTACKER_WAIT`, au lieu de 240 s).
Pièges : plafonner le duel de l'attaquant (100-240 s) cassait l'équilibre des batailles miroir
(`ai_battles_last_minutes_and_either_side_can_win` : attaquant 0-1/8) ; ouvrir l'assaut à tout
attaquant non inférieur raccourcissait trop les batailles (250 s < 300 s). La traversée de la
rivière n'était pas en cause (`dry_z` fait déjà traverser une ligne qui a les pieds dans l'eau).

## Animations (point 1)
Poses d'arc/arbalète trouvées par optimisation des positions des mains (script jetable, bras de
0,275 + 0,27 m, coude à 1,13 m ; bras = (flexion, abduction, coude), torsion, inclinaison). Pleine
allonge : torsion −0,9, gauche (−0,9, 2,1, 0,25), droite (−1,5, 1,25, −1,6). L'arc et l'arbalète en
visée sont posés dans le poing calculé par la chaîne du bras, dans un repère fixe (tangage du tir en
cloche, roulis). La décoche (s = 0,55 du cycle) est calée sur la volée du régiment (baisse des
munitions → `volley_time`), période = rechargement de la sim (6 s, 9 s Génois, 12 s engins).
Mêlée : un coup tiré au sort à chaque passe (garde 20 %, taille, revers, estoc), recul si « touché »,
choc de contact décroissant (`state_time`). Chutes : arrière/avant, côté, affaissement à genoux.

## Effets (point 2)
Tout est piloté par l'état de la sim (aucune règle) : volée = baisse de `ammo`, choc = passage
`charging` → `melee`. Budget fixe : 10 émetteurs de poussière, 6 d'éclaboussures (réaffectés chaque
image aux régiments en mouvement les plus proches, < 420 m), 3 × 8 gerbes ponctuelles, 3 072 traits
+ 64 boulets en tampons circulaires (trajectoire calculée dans le shader, transformée d'instance =
données). Pas de poussière sous la pluie ni la neige. Traits épaissis avec la distance (lisibles de
loin). Sang : **aucun** (public large).

## Captures (`docs/img/b4/`, avant = export de `ff592ae`)
`before_|after_` × `archers` (profil, pleine allonge), `crossbows` (réarmement), `charge`
(poussière), `melee`, `bombard` (éclair + fumée ; figurines hors simulation, la démo n'a pas de
bombarde), `battle_charge` (68 s), `battle_volley` (59 s) ; `after_volley_closeup`.

## Mesures (24/09, après fusion, en mêlée)
Nouvelle option `--bench-at=<s>` : avance la bataille (IA des deux camps) avant les 600 images mesurées.
Écran plafonné à 60 Hz malgré `--disable-vsync` : les FPS ne départagent pas ; on compare primitives et appels.

| Config (`--benchmark --bench-at=90`) | FPS moy. | Primitives | Appels | Projectiles |
|---|---|---|---|---|
| démo 14 unités | 58,8 | 1,38 M | 520 | 672 |
| démo, `--no-effects` | 58,7 | 1,22 M | 512 | 0 |
| `--camera=630,240,28,180` | 58,8 | 1,96 M | 751 | 804 |
| idem `--no-effects` | 58,7 | 1,80 M | 742 | 0 |
| `--units=20` (40 unités, 4553 soldats) | 58,6 | 2,87 M | 895 | 1995 |
| idem `--no-effects` | 58,5 | 2,71 M | 878 | 0 |

Conclusion : effets ≈ +6 % de primitives, +2 % d'appels, aucune baisse de FPS visible (sous le seuil de 15 %).
Piège : sous zsh, `$cfg` non cité n'est pas découpé ; utiliser `${=cfg}` dans les boucles de mesure.

## Suites (hors lot)
1. ~~Banc d'essai A/B~~ fait. Ancienne consigne : banc d'essai A/B (machine chargée : passes alternées) :
   `godot --path game --disable-vsync --resolution 1600x900 res://scenes/battle/battle.tscn -- --benchmark --units=20`
   et `... -- --benchmark --camera=630,240,28,180`, avec et sans `--no-effects` ; noter ici (seuil ~15 %).
2. Vérifier en jeu les éclaboussures (régiment en marche dans un gué ; jamais capturées en action).
3. Facultatif : fanions de lance verticaux à la charge (pivotent avec la lance couchée) ; capture
   de bombarde en siège (`--siege`) si l'armée en aligne.
