# ADR 0035 — Cartes de batailles historiques (Crécy, Poitiers, Azincourt)

Date : 2026-09-26. Statut : accepté. Lot EP7 du chantier « batailles épiques » (suivi :
`docs/wip/ep7-cartes-historiques.md`, `docs/wip/epic.md`). S'appuie sur EP1 (taille du champ, ADR
0076), EP2 (horizon, ADR 0032), EP3 (eau et routes, ADR 0033), EP6 (décor, ADR 0061), EP8 (heure,
ADR 0055) et EP9 (fin de bataille, ADR 0056).

## Contexte

Le joueur veut pouvoir livrer les trois grandes batailles de la guerre sur leur vrai terrain :
- Crécy, le 26 août 1346 ;
- Poitiers, le 19 septembre 1356 ;
- Azincourt, le 25 octobre 1415.

Chaque carte doit avoir :
- le relief réel, avec l'horizon du site ;
- un décor d'époque ;
- le déploiement historique : archers anglais et pieux, « batailles » françaises successives ;
- la météo et l'heure d'origine : averse en fin d'après-midi à Crécy, matin boueux à Azincourt.

Les cartes s'ouvrent depuis le menu, et depuis la campagne quand c'est possible.

L'IA des deux côtés doit rendre le résultat historique le plus probable, jamais certain.

Une contrainte de coordination s'ajoute : SG5 et EP9b travaillent dans `ai.rs` et `relief_ai.rs`, qui ne doivent pas être modifiés. Le placement historique doit donc être explicite, et l'IA ne doit pas le redéployer.

## Options

- **Scènes Godot faites à la main** : pas de règles testables, et la campagne ne peut pas les réutiliser.
- **Donner à l'IA un mode « historique »** : cela touche `ai.rs`, ce qui est interdit, et le comportement change avec chaque retouche de l'IA générale.
- **Données plus une couche de scénario dans le cœur, par-dessus l'IA existante.** La carte décrit le site et l'ordre de bataille. Une couche fine filtre ou complète les ordres de l'IA sans la modifier.

## Décision

**Données : `data/battle_maps/<id>.json`**, validé par `data/schemas/battle_map.schema.json`. Chaque carte contient :
- le site : lon/lat, cap de l'axe d'attaque et lieu ;
- le champ ;
- le relief, sous forme d'une grille de hauteurs en décimètres ;
- les bois, boues, mares, ruisseaux, chemins, haies et fossés ;
- un `DecorPlan` d'EP6 (`clear: true`) ;
- les armées, par blocs de régiments. Un bloc a une position, une orientation, une vague, un poste avec laisse, et peut préciser pied à terre, formation, moral, expérience et munitions ;
- les vagues : délai, vague précédente engagée, vague ennemie lâchée, assaut ;
- la météo et ses changements, et l'heure de départ ;
- le vainqueur historique ;
- une fenêtre de campagne (années) ;
- les sources.

**Relief :** `cent-ans geo battle-site` (`tools/cent_ans_tools/geo/battle_site.py`) calcule le relief :
- il échantillonne le Copernicus GLO-30 dans le repère tourné du champ ;
- il retire la canopée d'après ESA WorldCover, puis applique une ouverture de 90 m et un flou ;
- il écrit le résultat sur une seule ligne de la carte, pour que le reste du JSON reste lisible à la main.

L'outil cuit aussi la tuile d'horizon `hist_<id>` au format HZR1 d'EP2, déjà tournée, et un aperçu dans `docs/img/ep7/<id>_site.png`. Coût : 0 $ (données ouvertes déjà en cache).

**Cœur :**
- `sim-battle/src/historical.rs` fournit `HistoricalMap`, `battle_setup`, `apply_site` et `start`.
  - `apply_site` remplace le terrain généré par le site : relief, eau, bois et décor. La rivière, la côte et le village de B5 tirés au hasard sont retirés.
  - `start` règle la météo (`set_weather`) et l'heure (`set_start_hour`), puis pose le déploiement.
- `sim-battle/src/sim/scenario.rs` gère le scénario :
  - Déploiement explicite des blocs côte à côte, selon l'emprise réelle des régiments. Ils mettent pied à terre par les effets de l'ordre `dismount`. Aucun régiment n'est en réserve.
  - Vagues retenues. Elles sont lâchées au délai prévu, quand la vague précédente est engagée (la moitié de ses régiments en mêlée ou hors de combat), ou quand une vague ennemie est lâchée.
  - Vagues d'assaut : ordre d'attaque sur l'ennemi le plus proche, au pas de course sous 140 m.
  - Postes avec laisse : les Anglais tiennent la haie, le bois ou la crête. Les tireurs à court de flèches reviennent à leur poste.
  - Changements de météo en cours de bataille, et lignes de journal : « … s'ébranle. », fin de l'averse.
- `sim.rs::step` fait passer chaque ordre de l'IA par `scenario_filter`. Un ordre qui touche une unité retenue ou en assaut est écarté, ainsi qu'un déplacement hors de sa laisse pour une unité postée. `ai.rs` n'est pas modifié.

**Campagne :** si une bataille de campagne a lieu dans la province d'une carte et dans sa fenêtre d'années, `get_battle_setup` joint le site (`historical_site`). La scène appelle alors `apply_site` sur la bataille ordinaire : les armées de la campagne, déployées librement, se battent sur le vrai terrain, sans le scénario.

**Godot :**
- Le menu d'accueil gagne l'entrée « Batailles historiques » (`historical_battles_menu.gd`), avec trois choix par bataille : mener les Anglais, mener les Français, ou regarder.
- `battle_scene.gd` reçoit `--historical=<id>` et `--historical-side=`. La phase de déploiement est sautée. L'horizon utilise la tuile du site. Le ciel est rendu avec la météo finale, plus une averse qui cesse quand la simulation passe au sec.

## Équilibre

Les réglages portent sur les données seulement : écart entre les lignes, taille des ailes, flèches, effectifs et moral. Résultats en IA contre IA, sur 30 graines :

| Bataille | Victoires anglaises | Test (20 graines) |
|---|---|---|
| Crécy | 23/30 | 14 à 19 |
| Azincourt | 24/30 | 14 à 19 |
| Poitiers | 20/30 | 11 à 18 |

## Conséquences

- Une nouvelle bataille historique ne demande que des données : une carte JSON, un passage de `cent-ans geo battle-site` et un test.
- Les vagues françaises de l'IA sont scriptées en assaut. Seuls les ordres de l'IA sont filtrés. Le joueur commande librement tout son camp : il peut lancer une vague plus tôt, ou quitter la haie avec les Anglais. Dans ce cas, le script ne s'applique pas à son camp.
- Le site s'applique à toute bataille de campagne dans la province et les années de la carte, même si les armées ne ressemblent pas à celles de l'histoire.
- Le ciel de rendu est celui de la météo finale. Seule l'averse est dynamique.
