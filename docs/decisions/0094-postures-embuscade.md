# 0094 — Postures d'armée et ouverture d'embuscade (lot CV3-1)

Date : 2026-09-27. Statut : accepté. Spec : `docs/design/2026-09-27-campagne-vivante.md` § 1 et § 3.

## Contexte

La campagne ne connaissait que trois postures (`Normal`, `Raid`, `Siege`). La spec « campagne
vivante » ajoute l'embuscade, la marche forcée et le camp retranché, une bataille 3D qui s'ouvre
en embuscade (lot CV3-2) et des résultats de bataille nuancés. Il fallait décider où vivent les
règles, comment la bataille 3D apprend la situation, et d'où vient le « couvert » d'une case alors
que le core n'a pas de biome par case.

## Décision

1. **Un seul ordre.** `Order::SetStance` existant, étendu à `Ambush | ForcedMarch | Entrenched`
   (clés `ambush`, `forced_march`, `entrenched`), validé par `posture::validate_stance_change`
   (`OrderError::StanceRefused(raison FR)`). L'interface interroge `get_stance_options(army)` :
   posture → `""` ou raison du refus. Aucune règle en GDScript.
2. **Couvert d'une case = `data_model::CoverMap`**, échantillonnée à la résolution de la grille de
   navigation et décodée paresseusement une fois par processus (comme les rasters M2) :
   - forêt : la couverture forestière nommée par `map/forest_cover.json` (`splat.png`, canal B,
     forêts vers 1340) — et non `forest_kind.png`, qui ne donne que la part de résineux ;
   - marais : `map/wetlands.png` (max des canaux marais, étangs, prés humides), le raster de
     `wetlands.json` ;
   - bocage : terrain de la province ; sans rasters, repli sur le terrain de la province.
   Seuils et bonus par classe dans `data/rules/postures.json`.
3. **Déclenchement dans la marche.** Quand `march::simulate` arrête une marche dans la zone de
   contrôle d'une armée en `Ambush` en guerre avec elle, `posture::spring_ambush` tire la réussite
   avec le RNG de campagne. Réussite : `movement::fight_with_opening(embusqué, victime,
   BattleOpening::Ambush { victim: Defender })` — l'embusqué attaque. Échec : événement
   « embuscade éventée », l'embusqué est révélé et la victime l'attaque (bataille normale).
   Compétence d'embuscade = max(Commandement, `Intrigue` de `character_effects`) : pas de branche
   Intrigue.
4. **Contrat 3D minimal, porté par le setup.** `sim_battle::BattleSetup.opening: BattleOpening`
   (`Standard` par défaut, `Ambush { victim }`), `SideSetup.forced_march`, `SideSetup.entrenched`
   et `SideSetup.start_fatigue` (jauge 0-100, rempli depuis `postures.json`), tous en
   `serde(default)` et omis quand neutres : les batailles, replays et sauvegardes existants ne
   changent pas. `sim-battle` n'a pas à lire `postures.json`. `BattleRequest.opening` garde
   l'ouverture d'une bataille du joueur en attente.
5. **Résultats nuancés au point commun.** `battle_outcome::classify` (seuils de
   `data/rules/battle_outcome.json`) est appelé dans `movement::apply_battle_result`, commun à
   l'auto-résolution et à la 3D. Conséquences : multiplicateur d'XP du général
   (`dynasty::on_battle_resolved`), prestige du souverain, `Army.morale_modifiers` (décomptés en fin
   de tour, lus par `side_setup` et `Side.general_morale_bonus`), ligne de chronique.
   `CampaignState.last_battle_outcome` garde la dernière classification pour le pont
   (`get_last_battle_outcome`, clé `outcome` de `resolve_battle`).

## Conséquences

- Nouveaux champs d'`Army`, `BattleRequest`, `CampaignState`, `SideOutcome` en `serde(default)` :
  pas de changement de `STATE_VERSION`.
- Le premier appel à la couverture décode `splat.png` (4096² RGBA) et `wetlands.png` : coût unique
  d'environ une demi-seconde, seulement si une posture ou une embuscade est interrogée.
- Le placement en colonne et le déploiement sur les flancs restent au lot CV3-2 ; l'IA des postures
  au lot CV3-6 (l'IA actuelle n'en prend aucune).

## Suite CV3-2 (2026-09-27) : l'ouverture dans la bataille 3D

- Règles dans `data/rules/battle_opening.json` (schéma `battle_opening_rules.schema.json`,
  `sim_battle::OpeningRules`) ; placement dans `sim-battle/src/sim/opening.rs`, appelé après le
  déploiement par défaut, sans tirage aléatoire : un rejeu (format 1 inchangé) reconstruit la même
  ouverture depuis setup + graine.
- Embuscade : la victime en `Formation::Column` sur la plus longue route du champ (sinon l'axe
  long), avant-garde (cavalerie légère, premier tiers de l'infanterie) → bataille (cavalerie lourde,
  général, infanterie, tireurs, engins) → arrière-garde (dernier tiers de l'infanterie), tournée
  vers le bout de route opposé à son bord. Zones de l'embusqué : bandes alignées sur les axes de la
  carte, parallèles à l'axe dominant de la colonne, à `near_m`–`far_m` ; un ou deux flancs selon un
  score de couvert (forêt, haies/clôtures). `DeploymentZone` reste un rectangle aligné :
  `deployment_zones(side)` liste les zones, `deployment_zone(side)` rend la première.
- Pas de phase de déploiement pour la colonne ni pour un camp en marche forcée
  (`can_deploy`) ; `begin_deployment` rend `false` si c'est le camp du joueur, et l'IA ne
  redéploie ni eux ni l'embusqué.
- Camp retranché : `ObstacleKind::Palisade` (obstacle linéaire existant du site B5, donc déjà
  exporté au pont et pris en compte par les couverts, charges brisées et l'IA B6) posé devant la
  ligne ; ralentit (facteurs en données) et divise les pertes en mêlée du défenseur juste derrière.
  Pieux plantés d'office pour les unités `Stakes`.
