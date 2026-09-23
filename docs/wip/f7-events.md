# Lot F7b « Chronique enrichie » — état

Branche : `worktree-agent-a65b4684eae74eccc`. Tests : `core/crates/sim-campaign/tests/f7_events.rs`.

## Constat de départ
- 50 événements. Déjà présents (pas de doublon) : L'Écluse, Nicopolis, du Guesclin connétable, Cabochiens,
  mort du Prince Noir, assassinat de Louis d'Orléans (`evt_armagnacs_bourguignons`), Troyes, Jeanne d'Arc,
  Constance ; le Grand Schisme est géré par la religion (M5), pas par un événement.
- Campagne simulée (graine 1337, IA minimale pour tous, France jouée par l'IA) : la guerre franco-anglaise
  s'éteint vers 1355 et ne reprend jamais ; d'où des événements de reprise historiques (1369 appel des
  seigneurs gascons, 1415 Harfleur, 1449 Fougères) qui redéclarent la guerre.

## Points
1. [x] Squelette du test `f7_events.rs` (simulation 1337-1453).
2. [x] ≈ 40 nouveaux événements (27 historiques, 7 chaînés, 6 aléatoires).
3. [x] Chaînes : Tournai → Esplechin ; succession de Bretagne → Hennebont ; Neville's Cross → rançon de
   David II ; Rienzo → chute ; Nicopolis → rançon de Nevers ; Montereau → alliance anglo-bourguignonne →
   Troyes ; Jeanne d'Arc → sacre de Reims.
4. [x] Test : ≥ 20 nouveaux historiques déclenchés.
5. [x] Docs, build, smoke Godot vert (après `scope_allows` : un aléatoire réservé ne consomme plus de tirage pour les autres factions).

## Prochaine étape
Vérifier cargo test + clippy complets, puis rapport final.
