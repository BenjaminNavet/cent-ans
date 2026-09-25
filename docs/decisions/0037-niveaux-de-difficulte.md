# ADR 0037 — Niveaux de difficulté de la campagne

Date : 2026-09-25. Statut : accepté. Lot DF1, demande du joueur.

## Contexte

L'écran de choix de faction affichait une « Difficulté » par faction (`data/ui/front_end.json`,
`difficulty_labels`), purement indicative : rien dans `core/` ne dépendait d'un niveau choisi. Le
joueur veut, comme dans les campagnes de Total War, choisir un niveau (Facile, Normale, Difficile,
Très difficile) qui change réellement la partie : l'économie et le comportement de l'IA.

Contraintes : toute règle vit dans `core/` ; les données dans `data/` ; les empreintes et tests de
régression existants (équilibre M10/F4, auto-résolution N1, IA G5/DP1) ne doivent pas bouger ; les
anciennes sauvegardes doivent se charger ; la bataille 3D et l'auto-résolution doivent rester
cohérentes (une même bataille ne doit pas être plus facile à jouer qu'à résoudre, ou l'inverse).

## Options

- **A. Tricher sur les ressources de départ** (trésor, armées de 1337 plus ou moins garnis). Simple,
  mais l'effet s'efface en quelques tours et ne touche ni l'IA ni les batailles.
- **B. Une IA « plus intelligente » par niveau** (profondeur de planification, meilleures
  compositions). Idéal en théorie, mais l'IA stratégique (`ai::plan_turn`) n'a pas de réglage de
  qualité ; la dégrader artificiellement produit des comportements absurdes, et c'est un chantier.
- **C. Modificateurs chiffrés continus, lus dans les données** (ce que font les jeux de grande
  stratégie : revenus et entretien de l'IA, agitation du joueur, hostilité de l'IA, moral de ses
  armées contre le joueur). Chaque levier est un point d'accroche unique dans une règle existante ;
  « Normale » est neutre par construction.

## Décision

**Option C.** `data/rules/difficulty.json` (schéma `difficulty_rules.schema.json`) décrit quatre
niveaux `easy`, `normal`, `hard`, `very_hard`, avec libellé, description et huit modificateurs :

| Levier | Facile | Normale | Difficile | Très difficile |
|---|---|---|---|---|
| Revenus des IA (`ai_income_percent`) | 80 % | 100 % | 120 % | 140 % |
| Revenus du joueur (`player_income_percent`) | 115 % | 100 % | 100 % | 90 % |
| Entretien des armées IA (`ai_upkeep_percent`) | 100 % | 100 % | 90 % | 80 % |
| Coût de recrutement IA (`ai_recruit_cost_percent`) | 100 % | 100 % | 100 % | 85 % |
| Agitation des provinces du joueur (`player_unrest`) | −5 | 0 | +3 | +6 |
| Attitude des IA envers le joueur (`ai_attitude_to_player`) | +10 | 0 | −10 | −20 |
| Rapport de forces exigé pour déclarer la guerre au joueur (`ai_war_ratio_percent`) | 120 % | 100 % | 90 % | 80 % |
| Moral des armées IA contre le joueur (`ai_morale_vs_player`) | −5 | 0 | +5 | +10 |

Côté cœur (`sim-campaign/src/difficulty.rs`) : `CampaignState.difficulty` (enum `Difficulty`,
`#[serde(default)]` = `normal`, sans changement de `STATE_VERSION`) ; `set_difficulty` n'est
accepté qu'au tour 0 (niveau figé pour la campagne). Points d'accroche :

- revenus : fin de `faction_income_effective` (impôts et commerce après embargo ; l'administration,
  proportionnelle au revenu, suit) ;
- entretien : fin de `faction_upkeep` (armées et garnisons, avant la monnaie H5) ;
- recrutement : `recruit_cost` (l'IA planifie avec ce même coût) ;
- agitation : `resolve_population`, décalage de l'agitation visée des provinces contrôlées par le
  joueur ;
- hostilité : une raison « Niveau de difficulté » dans `attitude(IA → joueur)`, visible dans le
  panneau de diplomatie ; elle pèse sur la guerre, les alliances, les traités et l'acceptation des
  offres du joueur ;
- agressivité : `war_target` multiplie le rapport de forces exigé (prétendant ou opportuniste)
  quand la cible est le joueur ;
- batailles : le camp sans armée du joueur qui affronte un camp où il en a une reçoit le bonus de
  moral. En auto-résolution (`movement::auto_fight`, assauts et sorties de `siege.rs`, prévision
  `battle_forecast`) il s'ajoute à `general_morale_bonus` du `Side` ; en bataille 3D
  (`battle_setup`, `siege_battle_setup`) il s'ajoute au moral de chaque régiment IA, qui fixe aussi
  son plafond de moral dans `sim-battle` — exactement comme le bonus `ArmyMorale` d'un général. La
  prévision d'avant-bataille affiche la ligne « Niveau de difficulté : moral de l'IA +5 ».

Invariant : au niveau `normal`, chaque accroche est l'identité exacte (pourcentage 100 → valeur
inchangée sans arrondi, facteur 1,0, décalage 0), donc les tests et empreintes existants sont
inchangés ; `tests/df1_difficulty.rs` vérifie en plus qu'un tour complet donne la même sauvegarde.

Pont : `set_difficulty(id)`, `get_difficulty()`, `get_difficulty_levels()` (id, libellé,
description, effets chiffrés en français calculés par le cœur, niveau par défaut) ; la signature de
`new_campaign` ne change pas. UI : sélecteur à quatre boutons sous la fiche de faction (infobulle
parchemin : description et effets), l'ancien libellé par faction devient « Défi de la faction » ;
`SimFacade.pending_difficulty` est appliqué juste après `new_campaign` ; le niveau figure dans les
emplacements de sauvegarde et le menu pause.

## Conséquences

- Les niveaux se règlent sans recompiler (`data/rules/difficulty.json`) ; le défaut intégré
  (`DifficultyRules::default`) reproduit le fichier, vérifié par `data-model/tests/real_data.rs`.
- Les IA entre elles ne sont pas avantagées l'une par rapport à l'autre (revenus et entretien
  s'appliquent à toutes les IA, moral et hostilité seulement face au joueur) : l'équilibre du monde
  reste celui de Normale, décalé globalement contre le joueur.
- Limites : les batailles navales (`naval.rs`) ne reçoivent pas le bonus de moral ; l'IA ne joue
  pas « mieux », elle est seulement plus riche et plus hostile ; le mock (`CampaignSimMock`) retient
  le niveau sans l'appliquer. Les valeurs sont un premier réglage, à affiner après parties de test.
