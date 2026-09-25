# Lot BV2 — Bataille vivante (2) : morts, chutes, chocs de cavalerie, sang, démembrements

Branche `worktree-agent-af32527b47b35b47a` (a fusionné `main` à `fa7efb3c`, avec V2, AU1 et V3).
Backlog : `docs/audit/backlog-tw.md` § Bataille (idées J). ADR :
`docs/decisions/0022-chocs-morts-et-sang-rendus.md`.
Coordination : BV1 (flèches, sang au sol, taille des unités ; branche
`worktree-agent-a3fb69eecbaf4f8a1`, non fusionnée), V3 (feu, lumière), AU1 (audio).

## État
| Lot | État |
|---|---|
| 0. Squelette : cœur `impact.rs`, pont, `data/fx/battle_gore.json` + schéma + test | **fait** |
| 1. Morts et chutes (clip selon la cause, désarçonnés, chevaux abattus, cadavres par cellules, LOD, plafond) | **fait** |
| 2. Chocs de cavalerie (renversés projetés qui se relèvent, chevaux qui entrent dans la masse et ralentissent, piques et pieux) | **fait** |
| 3. Sang sur les figurines (taches progressives, gerbes, chevaux, réglage « Sang ») | **fait** |
| 4. Démembrements (réglage « complet ») | **fait** |
| 5. Corrections V2 : cadence de marche calée sur la vitesse, armes d'hast de la milice à deux mains | **fait** |
| Mesures FPS `--units=50`, captures `docs/audit/captures/bv2/` | en cours |

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

## Prochaine étape
Mesures A/B (`scratchpad/bv2/bench.sh`), captures finales, rapport.
