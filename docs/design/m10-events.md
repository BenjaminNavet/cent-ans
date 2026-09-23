# M10 (partie 1) — Événements historiques et aléatoires : spécification

Date : 2026-09-23. Objectif : un moteur d'événements piloté par les données, qui fait vivre la chronique de
la guerre de Cent Ans (grands événements datés, conditionnels) et des événements aléatoires à choix, avec
une fenêtre de décision pour le joueur. Design général : `docs/design/2026-09-23-cent-ans-design.md` § 5 (M10).

## 1. Données (`data/events/*.json`, `data/schemas/event.schema.json`)
Chaque événement : `id`, `title` (fr), `text` (fr, 2-4 phrases d'époque), `kind` (`historical` | `random`),
`trigger` :
- `date` (année, saison optionnelle) pour les historiques, ou `mean_time_to_happen` (tours) / `chance_permille`
  par tour pour les aléatoires ;
- `conditions` : liste de conditions typées (`faction_exists`, `faction_is_player`, `at_war {a, b}`,
  `controls {faction, province}`, `character_alive {id}`, `ruler_is {faction, character}`, `year_between`,
  `province_unrest_above`, `treasury_above {faction, amount}`, `religion_is`, `schism`, `not_fired {event}`,
  `ruler_trait {trait}`, `season`) ;
- `scope` : `global`, `faction` (une faction ciblée ou chaque faction qui remplit les conditions),
  `province` (tirée parmi celles qui remplissent) ;
- `options` : 1 à 3 choix, chacun avec `text` (fr), `effects` (liste typée : `treasury {amount}`,
  `unrest {province|all, amount}`, `population {province|all, percent}`, `health {…}`, `prestige {character|ruler,
  amount}`, `piety`, `papal_favor`, `opinion {faction, amount, reason}`, `declare_war {a, b}`, `peace {a, b}`,
  `add_trait {character, trait}`, `kill_character {id}`, `spawn_army {faction, province, units[]}`,
  `devastation {province, amount}`, `claim {faction, kind, target}`, `loyalty {vassal, amount}`) et
  `ai_weight` (pour le choix de l'IA).
Au moins 40 événements, sourcés (`sources`), dont une vingtaine d'historiques : Crécy/siège de Calais (1346-47,
si Angleterre et France en guerre), Peste noire (1347-1351, vague qui frappe la santé et la population de
toutes les provinces en progressant du sud vers le nord), traité de Brétigny (1360, si Jean II captif),
Jacquerie (1358), Étienne Marcel à Paris (1357-58), Grandes Compagnies (1360-1370), révolte des Paysans
anglais (1381), révolte des Ciompi (1378), les Maillotins (1382), folie de Charles VI (1392, si vivant et roi),
guerre civile Armagnacs et Bourguignons (1407+), Azincourt (1415), traité de Troyes (1420), Jeanne d'Arc
(1429, condition : Orléans assiégée ou France en difficulté), concile de Constance (lien avec le Schisme,
sans doublon avec M5), bataille de Castillon (1453). Une vingtaine d'aléatoires : bonne récolte, disette,
incendie de ville, foire prospère, pèlerinage, scandale à la cour, brigands, crue, épidémie locale,
don d'un marchand, défection d'un capitaine, miracle local, querelle de préséance…
Les historiques ne se déclenchent que si leurs conditions tiennent (l'histoire peut diverger) ; ils ne se
déclenchent qu'une fois.

## 2. Simulation (`sim-campaign`, nouveau `chronicle.rs`)
- `CampaignState` gagne `fired_events: BTreeSet<EventId>`, `pending_decisions: Vec<Decision{id, event,
  faction, province?, options[], expires_turn}>`. `state_version` : ne pas changer la valeur (4) si le merge
  se fait avant la sortie ; utiliser `#[serde(default)]` pour tous les nouveaux champs.
- Phase de fin de tour (après la religion, avant la population) : évalue les événements, tire les aléatoires
  avec le RNG de campagne (déterministe), applique immédiatement le choix de l'IA (option de plus fort
  `ai_weight`, départage par RNG) ; pour le joueur, crée une `Decision` (le premier choix s'applique
  d'office si elle expire au bout de 2 tours). Événements de journal `chronicle` (texte fr).
- Ordre `choose_event_option { decision, option }`.
- Effets appliqués via une fonction unique `apply_effect` validée ; un effet invalide (id inconnu) est ignoré
  avec un avertissement au chargement (validation dans `data-model`).
- La Peste noire : effet dédié `plague_wave { from_year, to_year }` qui touche les provinces par latitude
  (sud d'abord) sur ~12 tours : santé -30, population -15 à -30 %, mécontentement +20.
- Tests (≥ 10) : déclenchement par date, conditions non remplies (histoire divergente), une seule fois,
  aléatoire déterministe, choix IA, décision joueur puis ordre, expiration, peste noire, sauvegarde, validation
  des données (tous les événements chargent).

## 3. Pont et interface
- `CampaignSim.get_pending_decisions() -> [{id, title, text, options[{index, text, effects_text}], expires_in,
  province}]`, ordre `choose_event_option`.
- Godot : fenêtre « Chronique » parchemin (titre, texte, boutons d'options avec info-bulle des effets),
  ouverte en fin de tour quand une décision attend ; file si plusieurs. Nouvelle scène/script sous
  `game/scenes/ui/` et `game/scripts/ui/`, branchement minimal dans `campaign_map.gd` (bloc marqué M10).
- Journal : couleur dédiée `chronicle`.
- Smoke : 60 tours France, au moins un événement historique et un aléatoire, une décision résolue par ordre.
- Capture : `docs/img/godot-chronicle.png` (`--stage=chronicle`).

## 4. Critères de fin
Tests verts, smoke vert, capture relue, `docs/status.md`, `docs/godot-map.md`, `docs/design/data-model.md`
à jour.
