# Lot camprules (correctifs post-playtest 17/10)

Branche `fix/camprules`.

## État
1. Défaite en campagne, armées restées ensemble : à reproduire en jeu (le test core
   `tests/armies/fix_camprules.rs` passe : le vaincu quitte déjà Paris).
2. Emplacements 9/6 : la règle (ADR 0185 / WH econ) conserve ce qui dépasse déjà le plafond ; Paris démarre avec
   9 chaînes historiques pour un plafond de 6. Affichage corrigé (« 9/6 (au-delà du plafond, conservés) »),
   alerte « emplacement libre » renommée (elle compte des colonies sans chantier).
3. Entretien 0 : `faction_summary` lit `last_budget` (0 avant la première saison). Barre du haut (game-api) et
   Montre (client) lisent maintenant `faction_economy` (valeur vivante, identique au panneau d'économie).
4. « 213 provinces » : pas un défaut. La carte composée (CC, ADR 0357) compte 213 provinces
   (`data/provinces` = 213 fichiers = 213 entités de `provinces.geojson`) ; le ~440 est l'ancienne carte.
5. Eau construite deux fois : `build_water` tournait sur le sol plat puis `RebuildWater` (terrain prêt).
   `HeightSampler::is_ready` : pas de première construction tant que le terrain n'est pas prêt.

## Mise à jour finale
1. Cause réelle : le vaincu se repliait sur Saint-Denis (même province, quelques km) sans ligne au journal. Règle
   `retreat_target_after` : colonie amie hors de la province de la bataille d'abord ; journal « se replie sur ». ADR 0476,
   test `fix_camprules.rs`, `c7a_retreat` adapté. Vérifié en jeu : « se replie sur Orléans ». Butin/prisonniers : non prévus par le design, rien ajouté.
2-5 : voir plus haut. Eau : un seul « water: built » au démarrage (le second vient du retour de bataille).
Tests : core complet, clippy core+client, tests client OK ; smoke OK (3 échecs transitoires « Launch failed » relancés OK).
