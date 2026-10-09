# 0282 — Ligue anti-hégémon, clause « Rejoindre la guerre », appel d'un allié au joueur (lot WH `diplob`)

## Contexte
Le rapport critique WH (`docs/wip/wh/diplomatie.md` § 3, points 5, 6, 7) relève trois manques par rapport à Total War: Warhammer III :
aucune coalition contre une puissance qui domine la carte (la guerre permanente de l'IA laisse un premier courir) ;
aucune clause de traité désignant une tierce guerre (l'IA rejoint seule, sans que rien ne la pilote) ; et le joueur allié à une
IA attaquée entre en guerre d'office (`answers_call_to_arms` renvoyait `true`), sans choix ni conséquence.

## Décision
- **Ligue.** `CampaignState.league: Option<League { target, since_turn, until_turn }>` (`#[serde(default)]`), rafraîchie chaque
  saison par `update_league` (phase diplomatie). `hegemon()` : la faction vivante qui tient au moins `league.province_share`
  des provinces possédées et dont la puissance militaire dépasse `league.power_ratio` fois celle de la deuxième puissance
  (le joueur n'est visé que si `league.include_player`). Une ligue formée dure au moins `league.duration_turns` ; l'événement
  « Les princes se liguent contre X » n'est émis qu'à la formation (et « se dissout » à la fin). Effets, tous lus dans `data/ai/diplomacy.json` :
  attitude `league.attitude` de chacun (non allié, non vassal) envers l'hégémon (« Puissance menaçante pour tous ») ;
  `league.join_bonus` points à toute alliance entre deux ennemis de l'hégémon ; `plan_alliances` compte l'hégémon parmi les rivaux
  (cherche des partenaires deux fois plus souvent, attitude minimale `league.partner_attitude`) ; guerre contre l'hégémon sans autre motif
  (casus belli « ligue contre l'hégémon », donc sans malus d'agression) et rapport de forces exigé réduit de `league.war_ratio_factor`.
  Les membres ne cherchent qu'au plus `league.max_alliances` (1) alliance et la concluent en pacte défensif : sans ce plafond, la sonde
  (seuils abaissés à 5 % des provinces et ×1,3, pour forcer le déclenchement) montrait 641-705 guerres déclarées sur 120 saisons contre
  197-249 sans ligue, chaque alliance de plus ajoutant des appels aux armes ; avec le plafond de 1 : 304-421. Le facteur
  `war_ratio_factor` et le malus d'attitude n'y changent presque rien (mesuré, à alliances coupées : 245).
  Valeur `province_share` : 0,055 et non 0,18 de la spec, car le plus gros royaume de la sonde `campaign_probe` tient 29-31 provinces sur
  ~440 (7 %) : 18 % n'aurait jamais déclenché.
- **`Article::JoinWar { giver, target }`.** Le donneur déclare la guerre à `target`. Vérification : cible vivante, ni l'un des
  signataires, en guerre contre l'autre partie, pas déjà en guerre ni allié/vassal de la cible, ni trêve ni non-agression, trésor non négatif.
  Valeur (`treaty_weights.join_war`) : engagement, rapport de puissances de coalition (guerre favorable / cible écrasante), rancune,
  épuisement par la guerre, guerre ailleurs ; promettre de rejoindre *notre* guerre vaut `receives`. Application : `declare_war`
  (donc motif « défense d'un allié » et appel aux armes). Les raisons de refus sont les lignes de l'évaluation (`explain_treaty`).
- **Appel d'un allié au joueur.** `Article::AllyCall { aggressor }` (imposé, comme `Protection`). `call_to_arms` envoie une offre
  au lieu d'inscrire le joueur d'office (alliance simple ; le lien féodal garde sa règle). Accepter : `start_war` + événement ;
  refuser ou laisser expirer : mêmes effets que le refus d'une IA (`answer_call_shirked` : alliance rompue, rancune
  `ally_call.refuse_attitude`, historique `RefusedCall`). Extraction de `answer_call_joined` / `answer_call_shirked` : une seule décision pour l'IA et le joueur.

## Conséquences
- Les sauvegardes anciennes se chargent (`league`, offres : champs optionnels). Une offre `ally_call` ne contourne pas
  la vérification au moment d'accepter : si la guerre est finie, rien ne se passe.
- La ligue ne se déclenche presque jamais avec les seuils de la donnée (le premier tient 6-8 % des provinces et pèse 1,3-2,4 fois le deuxième) : c'est un filet de sécurité contre une vraie domination, pas un plafond de la part du premier (mesures dans le rapport du lot). La baisser augmente le nombre de guerres plus qu'elle ne borne le premier.
- Hors périmètre : l'IA n'envoie pas de `JoinWar` à ses alliés (ligne 10 du rapport).
