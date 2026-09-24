# Lot B8 — suites de bataille (relevées par B6/B7)

Plan : `docs/design/2026-09-24-rapprochement-total-war.md`, suivi `docs/wip/tw.md`. Suites de B4/B6/B7.

## État
| Point | État |
|---|---|
| 1a. `advance` incline vers un défenseur décalé | **fait**, testé |
| 1b. Cavalerie vs réseau de haies (bocage dense) | **fait** (régression) |
| 1c. Poursuite bornée (bocage + village, chevaliers vs archers montés) | **fait**, testé |
| 2. Minicarte : haies/fossés/village/côte | **non fait** |
| 3. Boue (carte de piétinement réutilisée) + sillage/gerbes de gué | **non fait** |
| 4. Infobulle des pastilles regroupées (liste des régiments) | **non fait** |
| Captures avant/après, banc perf | **non fait** |

Point 1 (IA Rust, `sim-battle`) terminé et testé ; points 2-4 (Godot : minicarte, boue/gué,
infobulle) et les captures/mesures n'ont pas été traités dans cette session — voir « Points
ouverts ».

## 1. IA (`core/crates/sim-battle/src/ai.rs`)

### 1a. `advance` incline vers un défenseur décalé
`advance()` : dans le cône avant (l'ennemi reste « devant »), la ligne penche désormais de
`dx.clamp(-ADVANCE_LEAN_MAX, ADVANCE_LEAN_MAX)` (20 m) vers le centroïde ennemi au lieu d'avancer
tout droit. Un défenseur pile en face (`dx` ~ 0) n'est pas affecté.

### 1b. Cavalerie vs réseau de haies
`detour()` réécrite : marche haie par haie (`detour_past`, jusqu'à `DETOUR_HOPS` = 4 sauts) au lieu
de ne tenter que les deux bouts de la première haie trouvée. Avant : si le point de sortie du
premier bout était encore bloqué par une haie voisine, le cavalier attendait indéfiniment (le filtre
final rejetait les deux candidats). Après : on avance bout par bout jusqu'à trouver un point
dégagé de tout obstacle vers la cible, ou on abandonne après 4 sauts (bocage vraiment trop dense).
Mise à pied non demandée : pas implémentée.

### 1c. Poursuite bornée
`plan_horse`, étape 4 (poursuite des unités en déroute) : nouvelle constante `PURSUIT_LEASH` (280 m).
Le cavalier ne s'engage à poursuivre que si la cible en déroute est à moins de `PURSUIT_LEASH` de
l'ancre de la ligne de bataille (`anchor`) ; au-delà, il retombe sur les étapes suivantes (retour
vers l'aile). Corrige la chasse des archers montés anglais par les chevaliers français en
bocage + village (contact retardé à 152 s).

## Choix consignés
- `battles_without_a_site_are_unchanged` (`core/crates/sim-battle/tests/b6.rs`) : digests mis à jour
  (seed 3 : `238 ... → 259 ...` ; seed 11 : `228 ... → 304 ...`). Justification dans le commentaire
  du test : les deux camps sont inégaux en nombre (6 unités françaises, 4 anglaises), donc leurs
  centroïdes ne sont jamais pile en face — le petit incliné de `advance` (1a) et la poursuite bornée
  (1c) changent le déroulé de ces champs mixtes (vainqueur inchangé : Attaquant). Vérifié séparément
  (`ADVANCE_LEAN_MAX = 0` en local) : la poursuite bornée à elle seule change déjà les digests ; les
  deux effets se cumulent. Un défenseur pile en face (les tests haie/fossé/village de B6) reste
  inchangé par construction (`dx` ~ 0 dans ces scénarios symétriques).
- `PURSUIT_LEASH = 280.0` : choisi empiriquement (voir mesures ci-dessous) — assez large pour ne pas
  couper les poursuites normales (un ennemi en déroute reste proche de la ligne), assez court pour
  ramener les chevaliers du bocage + village avant qu'ils ne s'égarent derrière la haie.
- `DETOUR_HOPS = 4` : le bocage de démo n'a jamais plus de 2-3 haies en chaîne ; 4 couvre une marge
  sans risquer un coût de calcul significatif (tests perf B4/B7 : primitives/appels, pas de budget CPU
  par tick mesuré en dérive).

## Tests
`core/crates/sim-battle/tests/b6.rs`, tous verts (`cargo test -p sim-battle`, 12 tests + 4 ignorés) :
- `battles_without_a_site_are_unchanged` : digests mis à jour, justifiés (voir Choix consignés).
- `bocage_village_seed_5_engages_near_seventy_seconds` (nouveau) : la graine 5 (bocage + village),
  pire cas relevé par B6 (contact à 152 s), engage maintenant à 66,6 s (< 90 s asserté). Mesuré sur
  les 8 graines de `bocage_battles_still_engage` : 60,6-112,3 s (avant : jusqu'à 152 s pour la
  graine 5), toutes < 180 s (test existant inchangé).
- `horse_leaves_a_rout_too_far_from_the_line` / `horse_still_chases_a_rout_within_the_leash`
  (nouveaux) : cavalier isolé, cible en déroute à respectivement 355 m et 155 m de l'ancre de ligne
  (`PURSUIT_LEASH` = 280 m) — abandonne au-delà, poursuit toujours en deçà. Piège rencontré : avec un
  seul ennemi en déroute (aucun autre « able »), `ai::plan` s'arrête avant même `plan_field`
  (garde `view.able_enemies().next().is_none()`) — les deux scénarios ajoutent un second défenseur
  hors de portée pour garder l'IA active.
- Réseau de haies (1b) : pas de test synthétique dédié (la contrainte « à ≤ 14 m de la cible » du
  filtre `HEDGE_COVER_REACH` rend une géométrie à la main peu lisible) ; couvert par la régression
  du contact bocage + village (graine 5 notamment), qui combine haie-réseau et poursuite.
- Rythme d'engagement démo : `demo_contact_stays_near_seventy_seconds` toujours vert (55-95 s).

## Prochaine étape (reprise)
1. Minicarte de bataille (`game/scripts/battle/` ou équivalent, à localiser — chercher le script
   qui dessine la minicarte à partir de `get_terrain()`/`site_label` du pont) : ajouter haies,
   fossés, village, côte, en s'appuyant sur les données déjà exposées côté Rust (B6, `site_label`
   dans `get_terrain()`, `get_site_label()`).
2. Boue : `game/scripts/battle/battle_terrain.gd` et `battle_ground.gdshader` (B7) — réutiliser la
   carte de piétinement (`trample_map`) pour un sol détrempé/pluie (teinte brune au lieu de neige
   tassée grise, cf. piste notée en fin de `docs/wip/b7-finitions-bataille.md`).
3. Gué : `battle_effects.gd` (B7, `_wet_span`) — sillage d'écume derrière les chevaux, gerbes plus
   fortes quand une charge entre dans l'eau (renforcer l'émetteur existant, pas le refaire).
4. Infobulle des pastilles : `battle_unit_markers.gd` (B7) — au survol d'une pastille regroupée,
   lister les régiments (déjà dénombrés pour l'affichage groupé) au lieu du texte actuel
   « N régiments — M hommes » seul.
5. Après 1-4 : `core/build.sh`, `godot --headless --path game --import` (si nouveau `class_name`),
   smoke test (compter « smoke OK », actuellement 23 sans régression B8 attendue côté Godot),
   captures `docs/img/b8/` avant/après pour chaque point visuel, banc
   `--benchmark --bench-at=90` (et `--units=20`) avant/après (primitives et appels, écran à 60 Hz).
