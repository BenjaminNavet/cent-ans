# 0779 — Histoire conditionnée et bifurcations

Statut : accepté (lot EVENTS, 23/10/2026).

## Contexte

Les juges d'histoire et d'UX (`out/juge/iter-02-histux.md`, `iter-03-histux.md` partie A) et de
design (`iter-03-design.md`) constatent que les événements-clés sont un script à date fixe :

- Brétigny annonce « l'Aquitaine en pleine souveraineté » et trois millions d'écus, mais ne
  transfère aucune province ;
- Poitiers prend le roi Jean II quoi que le joueur fasse, même si la France a gagné toutes les
  batailles ; Crécy n'existe que du côté français, sans lien avec une bataille jouée ;
- la clause de Jeanne d'Arc `provinces_below fac_france 22` est toujours vraie (la France n'a
  jamais plus de 22 provinces) ;
- aucune bifurcation : la partie à 1360 ressemble à celle de 1340.

## Décision

### Moteur (`core/`)

Conditions de `data/events/` ajoutées (schéma `event.schema.json`, `Condition` dans
`data-model`, évaluation dans `sim-campaign/src/chronicle.rs`) :

- `not` : négation d'une condition (jusqu'ici seul `any_of` combinait) ;
- `battle_balance { faction, rival, at_least }` : victoires de `faction` sur `rival` moins
  celles de `rival` sur `faction`. Le décompte vient de `ChronicleState::battle_tally`, alimenté
  par `battle_history::record` pour les seules batailles rangées (`BattleKind::Field`, résolues en
  3D ou automatiquement). L'historique `battle_history` de l'ADR 0157 est purgé et ne sert qu'à la
  carte ; le décompte, lui, est permanent et sauvegardé (absent des anciennes sauvegardes) ;
- `domain_below { faction, percent }` : part des provinces tenues en 1337 par la faction qui sont
  encore sous son contrôle (le « domaine royal » : 10 provinces pour la France) ;
- `strength_ratio { faction, rival, percent }` : hommes en armes comparés (équilibre des forces) ;
- `flag { flag }` et effet `set_flag { flag }` : mémoire durable d'un choix. Les drapeaux sont
  stockés dans `ChronicleState::flags` (sauvegardés) et lus par les événements postérieurs, qui
  peuvent aussi griser une option (`requires`).

Aucune règle d'économie ni de budget n'est touchée.

### Données

Chaque événement-clé a trois lectures exclusives selon l'état réel de la partie, avec
`not_fired` ou des conditions complémentaires pour qu'une seule se déclenche :

| Famille | Lectures | Conditions |
|---|---|---|
| Crécy (1346) | désastre français / triomphe anglais ; victoire française / revers anglais ; chevauchée sans bataille | `battle_balance` Angleterre ou France en tête ; égalité |
| Poitiers (1356) | roi Jean capturé ; prince de Galles capturé ; chevauchée sans bataille | idem, `ruler_is` Jean |
| Brétigny (1360) | A : roi captif, cessions réelles (Poitou, Saintonge, Agenais-Périgord-Quercy, Calais-Boulonnais) et rançon ; B : paix blanche ; C : Angleterre à bout (trésor et rapport de forces), Calais et Ponthieu rendus | `character_captive`, `strength_ratio`, `treasury_above` |
| Jeanne d'Arc (1429) | seulement si le domaine royal est sous 55 % **et** (traité de Troyes ou Touraine assiégée ou Anglais maîtres de l'Île-de-France ou de la Normandie) | `domain_below`, `fired`, `province_besieged`, `controls` |

Les scénarios A, B et C existent en deux versions : celle de la France (`scope` France,
conditions `not faction_is_player fac_england`) et celle de l'Angleterre (`faction_is_player
fac_england`), de sorte que le camp du joueur tient la plume du traité tandis que l'autre est
décidé par l'IA. Même principe pour Crécy (miroir anglais).

Équivalent pour les autres factions : `evt_sursaut_national` (aléatoire, 1,2 % par tour) pour
toute faction en guerre dont le domaine est sous 50 % : armée levée, mécontentement apaisé.

Trois chaînes à choix, avec drapeaux et conséquences durables :

1. **Castille** : mariage de Jean de Gand (1371, prétention au trône de Castille, drapeau
   `gand_roi_de_castille`) puis expédition de Galice (1386, guerre et armée), traité de Bayonne
   (1388, paix et indemnité ou poursuite de la guerre) et le point de vue castillan
   (`evt_castille_menacee`). Sans la prétention de 1371, rien ne suit.
2. **Bretagne** : la succession (1341) pose `bretagne_blois` ou `bretagne_montfort` ; Guérande
   (1365) pose `bretagne_hommage_france` ou `bretagne_alliance_anglaise` ; la confiscation du
   duché (1378) n'a lieu que dans le second cas (guerre France-Bretagne ou soumission).
3. **Étienne Marcel** : la Grande Ordonnance acceptée ou refusée (1357) ouvre ou ferme, à la fin
   de Marcel (1358), l'option de traiter avec Charles de Navarre (prétention au trône, guerre
   France-Navarre) ; l'option de rétablir l'ordre pose `dauphin_maitre_de_paris`, qui déclenche
   les réformes de Charles V (1364).

## Conséquences

- Une partie IA où la France gagne ses batailles ne voit plus Jean II capturé, et Brétigny
  n'enlève rien à la France sauf si elle a vraiment perdu la guerre. Le test
  `ev_history` établit chaque lecture sur état fabriqué.
- L'IA et le joueur passent par les mêmes conditions ; les poids d'IA des options privilégient
  l'issue historique (signer, accepter, revendiquer).
- Le décompte des batailles rangées ignore les victoires des alliés qui ne commandent pas
  l'armée principale (seule la faction de l'armée principale est créditée) : approximation
  assumée.
- Les événements dépendants de Jeanne (`evt_sacre_de_reims`, `evt_capture_de_jeanne_a_compiegne`,
  `evt_proces_de_rouen`) ne se déclenchent plus si la France tient bon : l'histoire de la
  Pucelle devient un recours, pas un décor. Le test `a_simulated_campaign_fires_the_new_history`
  (graine 1341, ≥ 20 événements) n'a pas été modifié ; la proportion tirée est mesurée dans
  `docs/wip/events-histoire-conditionnee.md`.
