# Lot G2 « IA : alignement historique » — état

Branche : `g2-ai-historical`. Périmètre : `core/crates/ai` (aucune modification de `sim-campaign` ni des
données : l'ordre `SendGift` existant sert aux subsides). Réglages : `docs/design/m9-ai.md` § 5 ; tableau
avant/après : `docs/status.md` (« Alignement historique G2 »). Tests : `core/crates/ai/tests/g2.rs` (4).

## Points
1. [x] Subsides (`ai/src/support.rs`) + pas de monnaie affaiblie si les bâtiments pèsent : Écosse 0,1-1,6 banqueroutes/décennie.
2. [x] Révolte de la laine et alliances dynastiques (`ai/src/alignment.rs`) : Flandre 40 %, Hainaut 38 % des tours de guerre ; Brabant 3 % (écart).
3. [~] Défection bourguignonne codée et testée ; jamais déclenchée (l'Angleterre ne tient jamais 8 provinces du royaume).
4. [~] Trésors : médiane 13,7 saisons (cible 8), 1-3 factions au-dessus de 8 ; pointes des royaumes assiégés.
5. [x] Docs, tests, `ai_probe` 0 % de refus côté France.

## Prochaine étape
Lot livré : fusion par l'orchestrateur. Pistes : offensives anglaises plus profondes (condition de Troyes),
mesure du trésor rapportée à l'entretien pour les royaumes assiégés.
