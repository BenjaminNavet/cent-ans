# 0034 — Porte-étendards, musiciens, étendards tombés et pris

Date : 2026-09-25 (lot EP5, chantier « batailles épiques », `docs/wip/epic.md`)

## Contexte
Le joueur veut que chaque unité porte un étendard, tenu par une figurine dédiée. Avant EP5
(BV3) : un drapeau collé à une figurine ordinaire du rang (`bearer_rank` 0,5), un nœud Godot
par régiment, affiché à moins de 220 m ; pas de modèle ni d'animation de porte-étendard, un seul
étendard par régiment, ni chute ni prise. EP1 porte en parallèle les batailles à 40 régiments et
plus par camp (15 000 soldats) : le coût par étendard doit rester minime.

## Décision
### Règles (cœur, `sim-battle/src/sim/standards.rs`)
- Chaque régiment (hors engins et bélier) porte un étendard : `Unit::standard` =
  `Carried | Fallen {x, z, timer} | Captured {by} | Lost {x, z}`.
- Chute : sous `heavy_losses_below` (50 %) de l'effectif complet, chaque pas où le régiment
  perd des hommes, probabilité `fall_chance_per_loss_percent` (1,5 %) par point d'effectif perdu ;
  débandade au contact : `rout_drop_chance` (35 %) ; le dernier homme tombe avec lui.
- Tombé : −6 moral d'un coup, −0,5 par seconde, coups de mêlée ×0,85 (cohésion : plus de point
  de ralliement) ; régiments ennemis à 60 m : +4 moral. Après `raise_seconds` (5 s) : pris si un
  régiment ennemi apte tient la place (15 m), relevé si le régiment tient (apte), sinon il attend
  (en déroute) ou reste au sol (`Lost`, régiment parti ou anéanti ; un ennemi qui passe le prend).
- Pris : −10 moral au régiment et plafond de moral −10 pour la bataille, −5 aux régiments du
  camp à 60 m, +8 au preneur ; effets doublés pour la bannière du général. Trophée
  (`StandardTrophy`) ; à la fin, le vainqueur ramasse les étendards restés au sol.
- Tirages sur un flux propre (`BattleRng::derive`) : les autres tirages d'une bataille sont
  inchangés (graines des tests intactes).
- Résultat : `SideResult::standards_taken` / `standards_lost` ; la campagne en fait une ligne de
  chronique (`GameEvent` Battle). `GeneralSetup::sovereign` : le général est le souverain en
  personne (bannière royale, oriflamme).
- Réglages : `data/rules/battle_standards.json` (schéma `battle_standard_rules.schema.json`),
  `BattleStandardRules::default` identique (test).

### Placement (cœur, rendu seulement)
`Unit::standard_slots(scale, bearers)` : rang du tampon de figurines qui porte l'étendard :
centre du premier rang, rang du milieu pour les tireurs en ligne (le premier rang doit voir),
pointe du coin ; deux porte-étendards (quart et trois quarts du front) à partir de
`two_bearers_from_soldiers` (120). Exposé par `get_units` (`bearer_slots`, `standard`,
`standard_x/y/z`, `standard_by`, `sovereign`).

### Figurines (Blender, ADR 0014)
`standard_0` (à pied), `standard_1` (à cheval), `musician_0` (tambourin), `musician_1` (busine),
sans arme ni écu, livrée du camp. Hampe de 3,8-4 m sur l'os virtuel `Prop`, tenue des deux mains.
Clips cuits dans les textures d'os existantes (ajoutés en fin, rien d'autre ne bouge) :
`std_idle/walk/run/wave/death`, `c_std_idle/walk/gallop/wave/death`, `drum_idle/march/beat`,
`horn_idle/walk/blow`. Manifeste : `pole_top` / `pole_axis` (pointe et axe de la hampe au repos).

### Rendu (`battle_standards.gd`, `battle_standard_flag.gdshader`)
- La figurine ordinaire du rang est masquée (`BattleSoldiers.reserved`, échelle nulle) et la
  figurine dédiée dessinée à sa place, clip en mode CUSTOM selon l'état du régiment.
- **Aucun nœud par drapeau** : MultiMesh par (camp, rôle, LOD) pour les figurines, par (rig,
  porté/à terre) pour les étoffes ; tampons rebâtis chaque image (≤ quelques centaines
  d'instances). Étoffes dans un `Texture2DArray` (une couche par étoffe distincte : pas de
  débordement des mipmaps d'un atlas).
- L'étoffe lit la matrice de `Prop` dans la texture d'os (même clip, même instant que la
  figurine) : elle suit la hampe quand le porteur marche, charge, brandit ou tombe. Elle flotte
  dans le vent de la bataille et grandit au loin (×3 de 140 à 700 m, émission relevée ; rien
  au-delà de 1 000 m) : les
  figurines lointaines restent des LOD2 (≈ 230 triangles), les imposteurs d'ADR 0024 ne portent
  pas de drapeau, les étendards restent lisibles jusqu'à ~600 m.
- Genres (`data/fx/battle_standards.json`) : pennon (bachelier, archers, sergents), bannière
  carrée (banneret, communes), étendard long à queue (retenues anglaises, compagnies
  d'ordonnance ; `fac_<id>_standard.png`, `banners.py`), bannière royale du général, oriflamme de
  Saint-Denis quand le roi de France est présent (ou « pas de quartier »), Saint-Georges et dragon
  pour l'Angleterre.
- Tombé : le porte-étendard joue `std_death` à l'endroit donné par le cœur, l'étoffe reste
  couchée au sol ; relevé ou pris : elle disparaît du sol (le mort reste). Pris : porté bas,
  étoffe renversée et sans vent, par une figurine du vainqueur à côté de son porte-étendard.
- Musiciens (données) à côté du porte-étendard ; sons par l'API de `BattleAudio.play_at`
  (« drum » en marche, « horn » à la charge, à la relève et à la prise).

## Équilibre
Premier réglage (60 %, 3 %) : l'IA battait moins sûrement une IA passive à forces égales
(`tests/ai.rs`, graine 0 : le camp passif gagnait). Sonde `ep5_standards::full_battles_see_standards_fall`
et balayage des réglages sur les graines 0-5 : 50 % et 1,5 % gardent les 12 duels IA / passif
gagnés par l'IA ; la sonde (6 batailles IA contre IA de 2 × 6 régiments) compte 14 étendards
tombés et 7 pris (premier réglage : 33 et 13), soit un régiment sur cinq. Les empreintes de `tests/b6.rs` (batailles sans site) sont recalculées (mêmes
vainqueurs, pertes à quelques hommes près). Graines de `tests/ai.rs` inchangées ; fixture de
calibration de l'auto-résolution inchangée (test vert).

## Conséquences
- `battle_soldiers.gd` : deux fonctions (`figure_at`, `figure_count`) et le masquage des rangs
  réservés ; aucune autre logique touchée (EP1 peut le retravailler).
- Repli sans figurine dédiée (`--rigid-figures`) : étoffe sur une hampe fixe à droite d'une
  figurine ordinaire, sans figurine masquée.
- Pas de clips de transition entre états (mode CUSTOM : changement sec de clip), pas de
  musicien à cheval.
