# 0327 — Croisade papale et prétention dynastique par mariage

Statut : accepté.

## Contexte
Lot TW `m2b` (`docs/wip/tw/campagne-m2.md` top5 et top7). Le pape ne dirigeait rien (la « croisade » du jeu est la faction `fac_crusaders`, ADR 0165) et un mariage entre deux couronnes ne donnait que +15 d'opinion, alors que la guerre de Cent Ans est dynastique. Une mécanique voisine existait : `diplomacy::upkeep::on_line_extinct` (prétention par la mère quand une lignée s'éteint).

## Décision
- Croisade papale (`papal_crusade.rs`, `CampaignState.papal_crusade`, `last_papal_call_end`) : à partir de `first_call_turn`, puis `interval_turns` après la fin du précédent appel, si le pape vit, il désigne une cible : de préférence une province de Terre sainte (`crusade.json` `holy_land`) tenue par un seigneur d'une autre foi que l'Église (ni catholique ni apparenté), sinon une province de foi chrétienne tenue par un tel seigneur. Événement `EventKind::Crusade`, fenêtre de `window_turns`.
- Un catholique qui déclare la guerre au détenteur de la cible (ou l'est déjà à l'appel) gagne `join_favor` de faveur et `join_prestige` de prestige, une fois. Le premier catholique qui tient la cible reçoit `reward_favor`, `reward_prestige` et `reward_gold` livres du trésor pontifical (plafonné), l'appel prend fin. Sans vainqueur à l'échéance, l'appel s'éteint (« sans suite »).
- IA : `diplomacy::ai::war_target` ajoute à l'ordre de priorité une branche croisade (priorité 500, sous une prétention, au-dessus de la guerre d'opportunité) pour un catholique non excommunié de faveur ≥ `ai_min_favor` et de puissance ≥ `ai_min_ratio` × celle de la coalition de la cible.
- Prétention par mariage (`dynasty::claim_by_marriage`, appelée à chaque naissance) : l'enfant, qui appartient à la faction de son père (ou de sa mère si elle règne), reçoit pour sa faction une prétention `ClaimKind::Throne` sur la faction de l'autre parent si celui-ci est de la maison régnante de sa faction (`dynastic_claim.require_ruling_house`), sans doublon. Elle donne le casus belli existant (`casus_belli`, « prétention au trône ») et pèse dans l'IA par `claim_stakes` / `main_claim` / `weariness_to_declare`, déjà branchés sur les `Claim`.
- Données : `data/rules/religion.json` (`papal_crusade`, `dynastic_claim`). UI minimale : ligne « Croisade du pape » dans la chancellerie (`get_religion_state` : `crusade_*`).

## Conséquences
- Écart à la spec : pas de bonus de moral temporaire des croisés (aucun `Effect` de moral par faction à durée limitée n'existe ; seul le moral de ferveur de `fac_crusaders`). Pas de ciblage des provinces d'un excommunié chrétien.
- Le rapport critique supposait la prétention « portée par la faction du père si l'enfant est héritier » ; elle est donnée dès la naissance à la faction de l'enfant (la lignée de l'héritier reste celle de la faction).
- Sauvegardes anciennes : champs en `serde(default)`.
