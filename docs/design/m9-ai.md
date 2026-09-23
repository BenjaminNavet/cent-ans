# M9 — IA de campagne et de bataille : spécification

Date : 2026-09-23. Objectif : remplacer l'IA minimale (M2) par une IA de campagne stratégique et donner
à la bataille temps réel (M7) une IA tactique. Design : `docs/design/2026-09-23-cent-ans-design.md` § 4.8.

## 1. IA de campagne (`core/crates/ai/src/campaign.rs`, `ai::plan_turn`)
Fonction pure `plan_turn(state, data, faction) -> Vec<Order>`, déterministe, utilisée par le pont
(`CampaignSim.end_turn` appelle `end_turn_with(ai::plan_turn)`) ; l'IA minimale reste celle des tests
de la simulation.
- **Évaluation stratégique** : ennemis actifs, puissance propre/coalition, menace par province amie
  (armées hostiles dans un rayon de 2 déplacements), fronts ; posture `offensive` (rapport ≥ 1,2 ou
  agressivité ≥ 60), `défensive` sinon.
- **Objectifs militaires** : chaque armée se voit attribuer au plus un objectif, par valeur décroissante :
  1. défendre une province amie menacée si l'armée peut l'atteindre et n'est pas écrasée (rapport ≥ 0,7) ;
  2. assiéger une province ennemie : valeur = revenu + 50 si capitale + 30 si prétention + 20 si déjà
     occupée par un allié à côté, divisée par (1 + distance), à condition que la puissance de l'armée dépasse
     1,5 × la défense ; poursuivre un siège en cours ;
  3. chevauchée (posture `raid`) en territoire ennemi riche si l'armée est trop faible pour assiéger et que
     la faction est agressive (Angleterre historiquement) ;
  4. regroupement : rejoindre la plus grosse armée amie à portée ; fusion des armées dans la même province ;
  5. retraite vers la capitale si l'armée est très affaiblie (effectif < 40 %) ou en territoire ennemi en
     hiver avec un ravitaillement < 30.
- **Économie** : budget d'entretien militaire ciblé (60 % du revenu en guerre, 30 % en paix), recrutement de
  la meilleure unité disponible (puissance/coût) dans les provinces qui peuvent recruter (capitale en
  priorité, puis provinces frontalières menacées), garnisons minimales dans les provinces frontalières ;
  construction du bâtiment de meilleur rendement (revenu, santé, ordre public si mécontentement élevé)
  quand le trésor dépasse une réserve de 2 tours d'entretien ; impôts : haut en guerre si mécontentement
  < 30, bas si mécontentement > 55 ; désarmement en cas de dette.
- **Personnages** : nommer des gouverneurs (meilleure gouvernance disponible) dans les provinces les plus
  riches, donner un général (meilleur commandement présent) à chaque armée sans chef, dépenser les points
  de compétence (branche selon le rôle), marier les membres adultes célibataires de la maison régnante
  (candidat de la même faction ou d'un allié).
- **Recherche et diplomatie** : réutilise `research::ai_choose_research` et `diplomacy::plan_diplomacy`.
- Tests (≥ 8) : une faction en guerre plus forte assiège, une province menacée est défendue, l'IA ne
  s'endette pas durablement, recrute dans le budget, nomme gouverneurs et généraux, déterminisme 20 tours,
  pas d'ordres invalides en boucle (taux de refus < 20 % sur 40 tours), 100 tours sans panique pour toutes
  les factions.

## 2. IA de bataille (`core/crates/ai/src/battle.rs`, après M7)
Plans par rôle d'unité pour `sim-battle` : ligne d'infanterie au centre, tireurs devant puis repli derrière
la ligne, cavalerie sur les ailes qui charge les flancs ou les tireurs isolés, réserve ; réactions : combler
une brèche, faire face à une attaque de flanc, retirer les unités en déroute, poursuivre un ennemi en
déroute avec la cavalerie ; posture défensive (pieux, colline) si le camp est plus faible (Crécy, Azincourt).
Commandes émises via l'API de commandes de `sim-battle`, toutes les 2 s simulées. Tests : l'IA bat une IA
passive à forces égales, la cavalerie charge un flanc exposé, les archers se replient au contact.

## 3. Critères de fin
Tests verts ; sonde `cargo run --release -p ai --example ai_probe` sur 100 tours : guerres menées, sièges
réussis, pas de faction en faillite durable ; smoke vert ; `docs/status.md` à jour.
