# Lot BV2 — Bataille vivante (2) : morts, chutes, chocs de cavalerie, sang, démembrements

Branche `worktree-agent-af32527b47b35b47a` (a fusionné `main` à `fa7efb3c`, avec V2, AU1 et V3).
Backlog : `docs/audit/backlog-tw.md` § Bataille (idées J). ADR :
`docs/decisions/0022-chocs-morts-et-sang-rendus.md`.
Coordination : BV1 (flèches, sang au sol, taille des unités ; branche
`worktree-agent-a3fb69eecbaf4f8a1`, non fusionnée), V3 (feu, lumière), AU1 (audio).

## État : terminé (non fusionné ; `main` fusionné à la fin, vérifications repassées)
| Lot | État |
|---|---|
| 0. Squelette : cœur `impact.rs`, pont, `data/fx/battle_gore.json` + schéma + test | **fait** |
| 1. Morts et chutes (clip selon la cause, désarçonnés, chevaux abattus, cadavres par cellules, LOD, plafond) | **fait** |
| 2. Chocs de cavalerie (renversés projetés qui se relèvent, chevaux qui entrent dans la masse et ralentissent, piques et pieux) | **fait** |
| 3. Sang sur les figurines (taches progressives, gerbes, chevaux, réglage « Sang ») | **fait** |
| 4. Démembrements (réglage « complet ») | **fait** |
| 5. Corrections V2 : cadence de marche calée sur la vitesse, armes d'hast de la milice à deux mains | **fait** |
| Mesures FPS `--units=50`, captures `docs/audit/captures/bv2/` | **fait** |

## Cœur (Rust)
- `core/crates/sim-battle/src/impact.rs` : `ImpactEvent` (attaquant, défenseur, sorte
  shock/pikes/stakes/broken, point de contact, cap, poids de charge, renversés, désarçonnés,
  profondeur, cohésion), `LossCause` ; règles déterministes (`charge_mass`, `knocked_count`,
  `pikes_stop`, `drive_depth`, `shock_morale`).
- Règles : renversés hors combat 3 s (`Unit::fighting_soldiers`) ; piques (schiltron) de front
  ou en carré : charge arrêtée, 8 % des cavaliers, moral −10. Cohésion = choc existant (8 / 15) :
  un malus supplémentaire cassait `ai_beats_a_passive_ai_at_equal_forces` (graine 2).
- `BattleSim::take_impacts()` (file bornée à 256) ; `Unit::loss_cause` / `loss_by` posés par les
  tirs, la mêlée (coup le plus lourd du tick ; « charge » pendant l'impact), pieux, piques, feu.
- Pont : `BattleSim.get_impacts()`, `get_units()` : `loss_cause`, `loss_by`, `knocked`.
- Tests `core/crates/sim-battle/tests/bv2.rs` (7) ; empreinte `b6.rs` graine 3 : 49 → 50.

## Godot
- `battle_soldiers.gd` : morts selon la cause (`deaths` des données), projection loin du tueur,
  cadavres par cellules de 80 m (LOD1 < 60 m, LOD2 au-delà, masqués > 380 m, 14 000 au plus) ;
  couche « renversés » (clip `knockdown`, place vidée dans la formation) ; chevaux qui entrent
  dans la masse (`_drive_in`) puis ralentissent / s'arrêtent (horloge locale du régiment) ;
  piques et armes d'hast abaissées devant une charge (`brace`) ; cadence de marche = vitesse
  réelle / vitesse nominale du clip (`cadence`) ; sang des vivants (`_living_blood`) ; signal
  `corpse_fallen` (accroche pour les traits fichés de BV1) ; sons via `BattleAudio.play_at`.
  `--no-bv2` : rendu d'avant (A/B).
- `battle_gore.gd` + `battle_gore.gdshader` : gouttes (quads, taches à plat au sol) et morceaux
  tranchés (tête, avant-bras, jambe) balistiques dans le shader, tampons circulaires.
- `battle_soldier_skinned.gdshader` : `INSTANCE_CUSTOM` = (instant, clip, projection m/s,
  code de partie + sang) ; parabole ; cheval qui détale (code 6) ; variante `BV2_CORPSE`
  (`BattleSkinned.corpse_shader()`) qui coupe la partie tranchée par `discard`.
- `battle_skinned.gd` : `c_fall` dans les morts montées, `death_index`, `clip_seconds`,
  `knockdown_config`, `sever_table` (os par partie), milice sur les clips de pique, `brace`.
- Réglage `battle/blood` (0/1/2, modéré par défaut) dans l'onglet Jeu ; BV1 crée la même clé
  dans un onglet « Bataille » : à la fusion, garder celui de BV1. `--blood=off|moderate|full`
  ou `0|1|2`.
- Blender : lance, vouge et fourche liées à l'os `Prop` (`battle_skinned_weapons.py`),
  `infantry_2` régénérée (`-- --no-rigs --only infantry_2`).

## Outils de test
- `game/tests/bv2_gore_shot.gd` : gros plans hors simulation (`--scene=deaths|sever|knock|horse|spray`,
  `--time=`, `--blood=`).
- `game/tests/bv2_charge_shot.gd` : charge réelle de chevaliers (`--target=unit_longbowmen`,
  `unit_flemish_pikemen`, `--stakes`, `--after=`, `--cam=`) ; en `--headless`, vérifie qu'un
  impact renverse des soldats (code de retour).

## Pièges
- Un shader qui écrit `POSITION` dans une branche doit l'écrire dans toutes (sinon sommets à 0).
- `PackedFloat32Array` lu depuis un `Dictionary` est copié à l'écriture : réaffecter
  `layer["data"] = data`.
- Un masque par sommet (démembrement) étire les triangles à cheval sur la coupure : couper au
  fragment (`discard`), seulement dans la variante cadavres.
- Après une fusion de `main` : `godot --headless --path game --import` (nouvelles classes).
- Répertoire temporaire partagé : scripts de BV2 dans `scratchpad/bv2/`.

## Mesures (`--disable-vsync --resolution 1600x900 -- --benchmark --bench-at=90 --units=50`, 11 800 soldats)
A/B alterné `--no-bv2` / BV2 (sang modéré) / BV2 `--blood=off`, machine partagée avec d'autres agents.
| Passe | `--no-bv2` | BV2 | BV2 sans sang |
|---|---|---|---|
| 1 (machine chargée) | 35,7 | 53,0 | 57,5 |
| 2 | 58,5 (méd. 60) | 58,5 (méd. 60) | 58,7 (méd. 60) |
| 3 | 58,6 (méd. 60) | 58,5 (méd. 60) | 56,7 (méd. 60) |
Primitives 1,90 M et 714 appels dans les trois cas. Écran plafonné à 60 Hz : pas de baisse
mesurable (seuil −10 % tenu). Une série précédente, avant deux optimisations (taches coupées
au-delà de 90 m, `discard` réservé aux démembrés), donnait −19 % en moyenne mais avec ±40 %
de bruit entre passes identiques.

## Captures (`docs/audit/captures/bv2/`)
`choc_archers_0..3` (0,3 / 0,8 / 1,6 / 3,2 s après l'impact : projetés, au sol, relevés),
`avant_choc_archers_0` (`--no-bv2`), `piques_0` (piques abaissées à l'impact), `piques_1`
(chevaux abattus), `pieux_0`, `choc_hommes_armes_0`, `gros_plan_{deaths,sever,knock,horse,spray}_complet`,
`gros_plan_sever_repos`, `gros_plan_deaths_{modere,sans_sang}`, `milice_deux_mains_melee`.

## Points ouverts
- Fusion avec BV1 : conflits attendus dans `settings.gd` / `settings_menu.gd` (clé `battle/blood`,
  garder l'onglet de BV1) et dans `battle_soldiers.gd` (nombre de figurines : `_unit_scale`
  prend déjà `n / soldiers`) ; brancher `corpse_fallen` sur les traits fichés de BV1.
- Vitesses nominales de `cadence` estimées à l'oeil (pas mesurées sur les clips).
- Renversés : retour à la place par un glissement court ; morceaux tranchés = primitives.
- Pas de GIF : séquences PNG seulement.
