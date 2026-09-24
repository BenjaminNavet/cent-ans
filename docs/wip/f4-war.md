# Lot F4 « Guerre de Cent Ans vivante » — état

Branche : `worktree-agent-a5777744f85f20632`. Sonde : `cargo run --release -p ai --example century_probe`
(5 graines × 464 tours en parallèle, tableau de synthèse ; `VERBOSE=1` pour le détail).
Tests : `core/crates/ai/tests/f4_war.rs` (7), `core/crates/sim-campaign/tests/f4_succession.rs` (4).
Réglages : `docs/design/m9-ai.md` § 4 ; tableau avant/après : `docs/status.md` (« Équilibrage F4 »).

## Points
1. [x] Guerre de prétention France-Angleterre : 73 % des tours [55-83], 5 à 8 phases.
2. [x] Alliances et appel aux armes : Auld Alliance 98-100 %, Bohême et Naples 100 %, Gueldre 81-100 %,
   Hainaut et Brabant selon les graines ; appels honorés 112-191, refusés 5-12. Flandre et Bourgogne ne
   passent jamais à l'Angleterre (écart).
3. [~] Trésors : 5-8 factions dépassent encore 8 saisons plus de 4 tours après 1350 (pointes à 13-25
   saisons : France, Castille, Empire) ; banqueroutes 0,92 / faction / décennie en moyenne (0,67-1,27) ;
   l'Écosse réduite à Fife vit à crédit.
4. [x] Mariages IA : 121-147 mariages entre factions par siècle.
5. [x] Batailles FR/EN 19-43 par décennie ; 375 provinces prises ; les 4 majeures vivantes en 1400 (5/5).
6. [x] Portugal : Pierre Ier ajouté (+ Amédée VI de Savoie), souverain vaincu tué à 1 %.
7. [x] Docs : `m9-ai.md` § 4, `status.md`.

## Prochaine étape
Lot livré (220 tests Rust, clippy, smoke Godot OK) : fusion par l’orchestrateur. Pistes : trésors des grandes couronnes après une paix lucrative
(dépense militaire plus rapide ou cour plus coûteuse), économie de l'Écosse réduite à une province,
défection bourguignonne (seuil de loyauté/score), Flandre révoltée alliée de l'Angleterre.
