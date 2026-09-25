# Lot BV1 — Bataille vivante (1) : volées, sang au sol, poussière, taille des unités

Branche `worktree-agent-a3fb69eecbaf4f8a1` (main fusionné en dernier à `821a4ceb` : V2, AU1, V3 et L1 compris). Backlog :
`docs/audit/backlog-tw.md` § Bataille (idées J). Coordination : V2 (soldats VAT) possède le maillage
et le shader des soldats, V3 l'environnement global et AU1 l'audio (bus, pool, banque). BV1 ne
touche à aucun des trois : un seul point de contact avec V2, le nombre de figurines dans
`battle_soldiers.gd`.

## État : terminé
| Lot | État |
|---|---|
| 0. Cœur : événements de tir + figurines par homme | **fait**, testé (`core/crates/sim-battle/tests/bv1.rs`) |
| 1. Volées massives, traits fichés, pieux et pavois, carreaux, feu, sons | **fait** |
| 2. Sang au sol (gerbes, flaques, éclaboussures, traînées) + réglage « Sang » | **fait** |
| 3. Poussière selon le sol et la météo, mottes sous les sabots | **fait** |
| 4. Taille des unités (réglage, ADR 0016, mesures Ultra) | **fait** |
| Captures `docs/audit/captures/bv1/`, bancs, smoke, `bv1_check` | **faits** |

## Cœur (Rust, règles inchangées)
- `sim-battle/src/shot.rs` : `ShotEvent` (instant, tireur, cible ou muraille, départ, visée,
  projectiles lâchés, tués, sorte flèche/carreau/boulet/pierre, feu, couvert pavois/pieux/mur).
  `BattleSim::take_shots()` : file bornée à 1 024, alimentée par `fire()` et `fire_at_wall()`.
- Feu : `shoots_fire` (`sim/fire.rs`) = assiégeant dont le type peut allumer un feu
  (`data/rules/siege_fire.json`, `ignition`). Sert au rendu seulement.
- `Unit::figure_count(k)` et `figure_positions(k)` : figurines serrées dans l'emprise simulée
  (ADR 0016). À k = 1, le résultat est identique à `soldier_positions()`.
- Pont : `get_shots()`, `set_figure_scale(k)` / `get_figure_scale()`, `get_units()[i].figures`,
  `pavise_cover`. `get_soldier_buffer` rend les figurines.
- Aucune empreinte de test modifiée : le multiplicateur n'agit que sur le rendu.

## Godot
- `battle_volleys.gd` (`BattleVolleys`, enfant de `BattleEffects`) + `battle_volley.gdshader` /
  `.gdshaderinc` : un paquet contient 256 traits, et le processeur écrit 16 flottants par paquet.
  Chaque volée compte 3 traits par homme simulé × la taille d'unité, 4 096 au plus. Tir
  étalé sur 1,5 s (0,8 s pour les carreaux), trajectoire en cloche calculée dans le shader,
  traînée de bougé en vol, épaissie de loin. Le hachage entier PCG est identique en GLSL et
  en GDScript (`arrow_landing`, vérifié par `bv1_check`).
- Traits fichés : un sur 5, au plus 96 par volée, passent dans une couche statique
  (`battle_stuck_arrow.gdshader`, 30 000 au plus, puis remplacement au hasard) et restent toute
  la bataille. Les autres restent fichés 8 s. Au-delà de 240 m, les traits fichés sont masqués.
- Pavois et pieux : sous couvert, les traits qui tombent devant la cible se fichent à 0,35-1,15 m
  dans la rangée de pavois (`pavise_cover`) ou de pieux (`stakes`). Ces rangées sont dessinées
  à l'avant des régiments et restent sur le champ.
- Carreaux : plus courts, plus épais, plus tendus. Flèches enflammées : pour une volée
  `incendiary`, la moitié des traits porte une flamme émissive, qui brûle encore 5 s une fois
  fichée.
- Sons : l'API d'AU1 (`BattleAudio.play_at` / `play_at_delayed`) joue le lâcher, le sifflement au
  tiers du vol et l'impact à l'arrivée, ainsi que la bombarde, le trébuchet et l'impact de
  pierre. `BattleAudio.auto_volley = false` évite le doublon avec les volées déduites des
  munitions.
- `battle_blood.gd` (`BattleBlood`) + `battle_blood_decal.gdshader` : gerbes GPU (16 émetteurs),
  décalques au sol (6 000 au plus : flaque qui s'étale, éclaboussures, traînée), qui passent du
  sang frais luisant au sang brun et mat en 90 s. Deux sources : les touches des volées (à
  l'arrivée du trait) et les pertes en mêlée (au premier rang). Traînées derrière les fuyards
  qui ont beaucoup saigné. Rien dans l'eau.
- Réglages (onglet « Bataille ») : `battle/blood` (Désactivé / Modéré / Complet, modéré par
  défaut) et `battle/unit_size` (Petite ×0,5 / Normale / Grande ×1,5 / Ultra ×2,5).
- Poussière : `BattleEffects.configure_ground(sol, météo)` ne lève de poussière que sur sol sec
  sans pluie ni neige, et la teinte selon le sol. Mottes (6 émetteurs) sous la cavalerie lancée,
  sur tout sol hors de l'eau : terre, boue lourde ou neige.
- Ligne de commande : `--no-bv1` (tout BV1 coupé, anciens traits B4 : mesures A/B),
  `--unit-size=<k>`, `--blood=<0|1|2>`. Le banc coupe la synchro verticale que `Settings`
  réimposait, ce qui plafonnait tous les bancs précédents à 60 Hz.
- Tests : `game/tests/bv1_check.gd` (sans rendu) ; `game/tests/bv1_shot.gd` (captures hors
  simulation : `--shot=volley|sky|stuck|bolts|fire|blood|dust`, `--ground=`, `--blood=`).

## Mesures (25/09, `--resolution 1600x900 -- --benchmark --bench-at=90 --units=20`)
La plus grosse bataille du banc compte 40 unités et 4 600 soldats. La machine était très chargée
(une trentaine de Godot d'autres agents en parallèle) : les séries alternent A/B et les écarts
entre passes atteignent ±30 %. Metal ne donne pas le temps GPU.

| Config | FPS moyen par passe | Primitives | Appels |
|---|---|---|---|
| sans BV1 (`--no-bv1`, traits B4) | 35,9 / 58,8 / 54,8 / 39,4 / 56,4 / 53,7 | 2,80-2,97 M | 837-850 |
| BV1, taille Normale | 58,7 / 58,8 / 58,8 / 46,6 / 59,1 / 65,9 | 3,00-3,16 M | 850-862 |
| BV1, taille Ultra (×2,5) | 60,7 / 57,4 / 42,3 / 45,6 / 37,6 / 45,4 | 5,84-6,27 M | 854-859 |

- Normale : aucune baisse, et même un léger gain sur chaque paire alternée (en moyenne 57,9
  contre 49,8). L'ancien système B4 renvoyait tout son tampon de 3 072 traits à chaque nouveau
  trait, alors que les paquets n'écrivent que 16 flottants. Primitives : +7 % (35 000 traits en
  vol et 7 000 fichés sur la minute mesurée).
- Ultra : primitives ×2,1, et en moyenne −20 à −25 % de FPS par rapport à Normale (≈ 45 FPS
  sur cette bataille chargée).
- « Avant » sur machine calme (début de session) : 99 FPS en moyenne pour `--units=20` et 122 pour
  la démo.

### Après fusion de V2, AU1 et V3 (figurines skinnées, flammes dans une couche à part)
Écran plafonné à 60 Hz (la synchro verticale est revenue avec la fusion). Temps de rendu CPU
fourni par V3 ; Metal renvoie 0 pour le temps GPU.
| Config (`--units=20`) | FPS (2 passes) | Primitives | Appels | Rendu CPU |
|---|---|---|---|---|
| sans BV1 | 58,7 / 58,7 | 2,43 M | 890 | 0,70-0,72 ms |
| Normale | 58,4 / 58,7 | 2,61 M (+7 %) | 906 | 0,71-0,73 ms |
| Ultra | 55,5 / 58,7 | 4,86 M (×2) | 907-911 | 0,75-0,90 ms |
Série non plafonnée, avant cette fusion (4 paires alternées, machine chargée) : sans BV1 35,6 /
44,7 / 81,4 / 43,3 et Normale 35,1 / 44,6 / 37,7 / 54,7. Parité hors la passe aberrante à 81,4.
Sans rendu (headless), les deux configs tournent à 135,7 FPS : BV1 n'a pas de coût CPU mesurable.

## Captures (`docs/audit/captures/bv1/`)
`avant_bataille_volee` / `apres_bataille_volee` (`--shot-at=59 --camera=600,360,45,200`, avec ou
sans `--no-bv1`) ; `taille_normale` / `taille_ultra` (`--shot-at=50 --camera=640,470,70,200`) ;
hors simulation : `apres_volley`, `apres_sky` (pluie de flèches vue d'en dessous), `apres_stuck`
(une minute de tir, pieux), `apres_bolts` (carreaux dans les pavois), `apres_fire`,
`apres_blood` (complet), `apres_modere_blood`, `apres_dust` (sol sec), `apres_boue_dust` (boue :
mottes sans poussière).

## Pièges
- Le répertoire de travail temporaire est partagé avec d'autres agents : les scripts de BV1 sont
  dans `scratchpad/bv1/`, car un `shots.sh` générique a été écrasé.
- `ParticleProcessMaterial` : une branche `elif` mal indentée (remplacement raté) laissait les
  mottes en émission sphérique ponctuelle. Vérifier `emission_shape` en cas de doute.
- Dans un script `SceneTree`, les nœuds ne sont pas encore dans l'arbre au moment de `_init` :
  utiliser `position`, pas `global_position`.
- Démo : la mêlée se joue dans la rivière, où le sang au sol est volontairement absent.

## Pistes (BV2 et suivants)
- Sang sur les figurines, démembrements, traits fichés dans les corps et les cadavres (BV2 : les
  touches sont déjà disponibles via `BattleEffects.hit_landed`).
- Pavois : les Génois portent déjà le leur dans le dos (V2), ce qui le montre en double quand la
  rangée est plantée.
- Ultra : imposteurs au-delà de 300 m pour récupérer les 20-25 %.
