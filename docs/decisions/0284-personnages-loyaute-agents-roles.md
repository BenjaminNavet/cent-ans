# 0284 — Loyauté des personnages, attentats d'agents, compétences par rôle, déclencheurs de traits

Statut : accepté

## Contexte
Écarts #3, #5, #8, #9, #14, #15, #19 de `docs/wip/wh/personnages.md` (top 4, 5, 7, 8, 10). Vérifié dans le code : `CharacterState.loyalty`
existait (défaut 100, seulement relevé par les ordres de chevalerie) sans aucun lecteur ; les agents n'avaient aucune action sur un
personnage ni sur une armée ; les traits `vétéran`, `maître de siège`, `cruel` étaient codés en dur dans `dynasty.rs` avec leurs seuils
dans `dynasty.json` ; l'arbre de compétences était le même pour tous. Le rapport disait vrai sur tous ces points.

## Décision
- **Déclencheurs de traits** (`data/rules/trait_triggers.json`, `trait_triggers.schema.json`, `sim-campaign/src/trait_triggers.rs`) :
  chaque entrée donne un trait si toutes les conditions de `when` (compteurs de batailles, sièges, chevauchées, âge, piété, trésor
  de la faction, dernière bataille gagnée) et un rôle de `roles` sont vrais. Évalué après chaque bataille, siège et chevauchée, et à
  chaque saison (`resolve_characters`), sans tirage aléatoire. Les trois traits codés en dur et leurs seuils quittent `dynasty.rs` /
  `dynasty.json` ; le trait blessé (aléatoire) y reste. Six traits ajoutés : héros de bataille, fléau des sièges, pillard, dévot,
  endetté (temporaire, 8 saisons), vieillesse (60 ans).
- **Compétences par rôle** : `Skill.requires_role` (`ruler`, `general`, `governor`, `consort`, un seul suffit ; vide = tous).
  `skills::roles_of` dérive les rôles de l'état (souverain de sa faction, épouse du souverain, armée commandée, province
  gouvernée) ; `learnable_skills` et `learn_skill` (erreur `WrongRole`) filtrent. Douze compétences de spécialité (4 général,
  2 gouverneur, 3 souverain, 3 épouse) greffées sur des racines communes. Le pont ajoute `requires_role` et « Réservé : … » à la
  description.
- **Loyauté** (`data/rules/loyalty.json`, `sim-campaign/src/loyalty.rs`) : chaque saison, la loyauté des généraux et gouverneurs
  non souverains glisse de `drift` vers `base_target` + effet `Loyalty` des traits/compétences × `effect_weight` + bonus d'ordre de
  chevalerie − province en mécontentement − trésor négatif − défaite récente. Sous `defect_below` : tirage dérivé (pas le flux
  principal) à `defect_permille` ‰ par saison ; un général emmène son armée chez l'ennemi en guerre le plus étendu (sans ennemi ou
  armée en siège : il ronchonne, loyauté remise à `recover_to`) ; un gouverneur perd sa province qui s'échauffe (`defect_unrest`).
  Au-dessus de `high_above`, +`high_morale` de moral aux troupes du général. Événement `Loyalty`, fiche : pastille « Loyauté ».
- **Attentats et aide aux armées** : quatre actions dans `data/rules/agents.json` : `assassinate` et `poison` (espion, effet
  `strike` : mort `kill_percent` (+ par sceau), sinon blessure `trait_wounded`, scandale d'opinion du maître des lieux pour
  l'assassinat seul ; cible = général posté à portée ou gouverneur de la province, le souverain seulement au sceau
  `target_ruler_min_level` = 4 ; la mort passe par `characters::kill`, donc la succession est résolue) ; `guide_army` (héraut :
  armées amies dans `army_reach_km` marchent `movement_percent` de plus et leur province est mise en vue) ; `ambush` (espion :
  armées ennemies dans le rayon perdent du mouvement et du moral, modificateur temporaire). Jets dérivés (`derived_rng`).

## Conséquences
- L'IA n'utilise pas encore les quatre nouvelles actions (le joueur seul) : pas de décalage du flux aléatoire côté agents.
- Écarts avec le rapport : pas de baisse de loyauté sur « rançon refusée » ni « titre accordé à un rival » (aucun événement à
  accrocher dans le code : la rançon refusée est une décision du geôlier, pas du chef du captif) ; le scandale d'un assassinat
  n'atteint que la faction de la victime, pas « tous » ; `poison_wells` du tableau devient `poison` visant un personnage.
- Loyauté initiale 100 (défaut historique) : les premières saisons voient le bonus de moral au-dessus de 90 pour tous les
  généraux, puis il disparaît (cibles de 65 en moyenne).
- Nouveaux champs sérialisés avec `#[serde(default)]`, valeurs dans `data/`.
