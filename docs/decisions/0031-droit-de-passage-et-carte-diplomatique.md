# ADR 0031 — Droit de passage, carte diplomatique et refus expliqués (lot DP2)

Date : 2026-09-25. Statut : accepté.

## Contexte

Après DP1 (ADR 0025), l'accès militaire n'était qu'une clause de ravitaillement : une armée pouvait
camper sur les terres de n'importe quel voisin en paix sans conséquence, et l'IA traversait les
terres neutres comme les siennes. La carte diplomatique (touche N) ne distinguait que la relation
formelle (paix, trêve, alliance, guerre, vassal), la minicarte restait politique, et la négociation
n'affichait qu'une ligne de « considérations » : le joueur ne savait pas quel point faisait
échouer son traité. C5R avait noté la faible valeur d'un accord commercial avec l'Angleterre.

## Décision

1. **Droit de passage = clause `military_access` de DP1** (pas de nouvel article). Une armée
   **finissant sa saison** sur une province contrôlée par une faction en paix ou en trêve avec la
   sienne, ni alliée, ni vassale, ni suzeraine, sans accès militaire accordé, **commet une
   intrusion** (`sim-campaign/src/passage.rs`, `trespassed_owner`).
2. **Détection en fin de tour**, une fois par saison (`turn.rs`, avant `resolve_diplomacy`), sur la
   position des armées : `movement.rs` n'est pas modifié (le test « depuis la fin du
   déplacement » est inutile, la fin de saison suffit et reste déterministe). La traversée en
   cours de marche n'est pas punie ; c'est le campement qui l'est (à la Total War : on compte les
   tours passés sur les terres d'autrui).
3. **Incident** : modificateur d'opinion unique « Violation de nos terres » chez la victime, remplacé
   chaque saison : `base_penalty` (10) + `per_season_penalty` (5) par saison consécutive,
   au plus `max_penalty` (40), oublié `memory_turns` (12) saisons après le départ.
   Registre : `DiplomaticLedger::trespassers` (saisons consécutives, total, provinces).
4. **Casus belli** : dès `casus_belli_seasons` (2) saisons consécutives, la victime tient un casus
   belli « violation de nos frontières » (`CampaignState::casus_belli`) pendant `memory_turns` :
   déclarer la guerre ne lui coûte plus le malus d'agression, et l'IA agressive s'en sert (G5).
5. **Trêve** : `truce_grace_seasons` (2) saisons de grâce pour évacuer après une paix.
6. **IA** (`ai/src/grid.rs`) : les places des terres qu'elle ne veut pas violer sont retirées de son
   graphe de marche (ni atteintes ni traversées). `passage::ai_may_trespass` : jamais en paix ;
   en guerre, si son agressivité ≥ `ai_violate_aggression` (70 : l'Angleterre), si elle hait le
   propriétaire (attitude ≤ `ai_violate_attitude`, −40), ou s'il est deux fois plus faible
   (`ai_violate_power_ratio`) et mal vu. Les accès militaires réciproques de DP1
   (`ai::diplomacy_eval`) ouvrent le passage.
7. **Avertissement avant l'ordre** : `CampaignSim.find_path_trespass` ; le chemin M4 passe en rouge
   et l'infobulle dit « ⚠ Sans droit de passage chez … ».
8. **Carte diplomatique** (`stance.rs`) : allié (vert), accord (bleu : commerce, accès militaire ou
   mariage), neutre (jaune), tension (orange : attitude envers nous ≤ `tension_attitude` −20,
   embargo, intrusion ou casus belli d'intrusion), guerre (rouge), vassal ou suzerain (gris).
   Même palette (`game/scripts/ui/diplomatic_stances.gd`) pour le filtre « Diplomatie » de MF1
   (ADR 0048, `MapModeController` : carte 3D, minicarte qui suit le filtre actif, légende et
   infobulle de survol), la légende UX1 (`data/ui/map_legend.json`) et la carte du panneau de
   diplomatie. Pas de bouton à part dans la minicarte.
9. **Refus expliqués** (`treaty_explain.rs`, `CampaignSim.explain_treaty`) : chaque raison de
   l'évaluation DP1 devient une ligne pondérée (« Ils se méfient de vous −12 », « Accord
   commercial — Routes commerciales communes +8 »), objections d'abord. Quand **un seul point**
   bloque, il est nommé et une **contre-offre** est calculée : l'exigence d'or ou de tribut
   ramenée à 75, 50 ou 25 %, sinon retirée ; pour une considération générale (attitude,
   confiance…), la compensation de `counter_proposal`. Plusieurs points : pas de contre-offre
   automatique (le bouton « Que faudrait-il ? » reste).
10. Réglages : `data/ai/diplomacy.json` § `passage` (schéma à jour) ; `enabled: false` (défaut sans
    la section) rend l'ancien comportement.

## Accord commercial avec un rival (point 4)

Vérifié et voulu. France → Angleterre en paix, au départ : « Commerce » +6, « Routes commerciales
communes » +8, mais « Enrichir un rival » −12, l'attitude (÷ 3, −22 en 1337) et la prudence −3 :
environ 5 % de chances. Le malus de rival (ADR 0025) traduit la guerre économique du temps
(embargo d'Édouard III sur les laines vers la Flandre en 1336, étape de Calais contre les draps
français) : deux couronnes rivales ne s'enrichissent pas l'une l'autre. L'accord reste possible
avec une attitude redressée (présents, mariage) ou une compensation (or, tribut), que la
contre-offre propose quand le rival est le seul point bloquant. Test :
`dp2_explain::a_trade_agreement_with_a_rival_is_worth_little_on_purpose`.

## Conséquences

- Pas de changement de `STATE_VERSION` : `trespassers` est `#[serde(default)]`.
- L'IA contourne les terres neutres. Sonde `balance_probe campaign 200 1-8` (règle coupée → active) :
  guerre France-Angleterre 52 → 57 % des tours, changements de propriétaire 21,1 → 16,6, batailles
  299 → 242, banqueroutes 0,51 → 0,29 par faction et décennie ; milice, révoltes et mécontentement
  inchangés. La carte bouge un peu moins : à surveiller.
- Limites : une traversée sans halte n'est pas punie ; le graphe de l'IA exclut des places, mais le
  pas de grille entre deux places peut frôler une province neutre.
