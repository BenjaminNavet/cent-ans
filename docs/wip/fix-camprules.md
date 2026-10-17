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
