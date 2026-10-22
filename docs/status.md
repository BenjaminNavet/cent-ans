# État du projet

Dernière mise à jour : 2026-10-22. Référence : branche `main` (a8effdd93). Historique jalon par jalon
jusqu'à M10 / F1-F9 / G1-G5 : `docs/archive/status-historique.md` ; feuille de route : `docs/roadmap.md`.

## En bref
- **Jeu** : campagne 1337-1453 (1477 pour la Bourgogne) jouable de bout en bout ; batailles et sièges en 3D,
  arbitrage automatique des batailles navales (pas de combat naval 3D, décision du joueur) ; sauvegardes,
  tutoriel, encyclopédie, codex, manuel (`docs/manuel.md`, raccourcis § 15 alignés sur le code le 22/10).
- **Client** : **Bevy 0.20** (ADR 0357). Le client Godot et le pont GDExtension sont retirés (ADR 0366,
  fusion du 2026-10-17 dans `main`). Lancement : `tools/launch.sh` (ADR 0362) ; sous Windows, lanceur
  `Lancer Cent Ans.exe` (ADR 0153).
- **Noyau** : Rust dans `core/` (crates `ai`, `data-model`, `game-api`, `relief-lod`, `seed-bots`,
  `sim-battle`, `sim-campaign`, `vegetation`). Les règles de jeu y vivent ; `client/` ne fait que rendu,
  UI et entrées.
- **Dépôt** : `main` est **996 commits devant `origin/main`** (dernier push le 2026-10-14). Le joueur a
  interdit le push ; aucune sauvegarde distante à ce jour.
- **Playtest** : 7 testeurs le 2026-10-17, 11 lots de correctifs fusionnés (`docs/wip/fix-playtest-1017.md`
  pour les restes). La nuit de playtest du 2026-10-22 (`docs/wip/nuit-playtest-2026-10-22.md`) n'a pas
  encore de constats.
- **Critères de sortie** : aucun. Il n'existe pas encore de définition écrite d'une version jouable.

## Budget (à décider par le joueur)
- **Dépensé : 136,39 $** (OpenRouter 59,82 $ ; fal.ai 76,57 $), d'après `cent-ans budget summary` lancé le
  2026-10-22 sur `docs/budget.md`.
- **Plafond v1 de 50 $ : dépassé, environ 2,7 fois.** Le plafond d'origine n'a pas été relevé ; les
  dépenses ont été approuvées par enveloppes nommées. Un plafond global révisé doit être fixé par le joueur.
- La synthèse en tête de `docs/budget.md` (2026-10-09, 117,39 $) n'est plus à jour : elle omet notamment
  TX (11,82 $) et le banc Meshy MX (6,90 $). À régénérer.
- Crédits Meshy (abonnement, hors plafond) : solde de **125 crédits** d'après le journal du 2026-10-14.

## Chantiers récents (2026-10-13 → 2026-10-22)

| Chantier | État | Où lire |
|---|---|---|
| Migration Bevy (dépôt Godot retiré) | fusionné dans `main` le 17/10, non poussé | ADR 0357, 0362, 0366 |
| Playtest 17/10 : correctifs | 11 lots fusionnés | `docs/wip/fix-playtest-1017.md` |
| Qualité visuelle (VQ) | 8 lots fusionnés le 17/10 ; cible 40 i/s en ultra **non atteinte** et non remesurée depuis | `docs/wip/vq-qualite-visuelle.md` |
| Seed bots | ADR 0348 fusionné le 13/10 ; correctifs ADR 0350 ; métriques de révoltes le 22/10 | ADR 0348, `docs/wip/seed-bots.md` |
| Choc des charges en données | ADR 0711 fusionné le 20/10 ; paramètres « neutres » tant que le rééquilibrage IA n'est pas fait | ADR 0711, `docs/wip/choc-charges.md` |
| Style Total War (TS) | lots T1 à T8 fusionnés le 20/10 ; T9 et T10 cités dans les notes, état à recouper | `docs/wip/ts-style-total-war.md` |
| Refonte des systèmes | lot 1 A fait (mesure des révoltes) ; mêlée, économie et IA de campagne non commencées | `docs/wip/refonte-systemes.md` |
| Branches `bl/*` (backlog) | en pause depuis le 17/10, hors `main` ; tri ou abandon à décider | `docs/wip/bl-backlog.md` (branche `bl/integ`) |
| Meshy MX | vagues 11-13 dans `main` ; publication des modèles bloquée | `docs/wip/mx-meshy.md` |
| Portage Windows | code fait (ADR 0087, 0153) ; essai sur un vrai PC restant | `docs/wip/windows.md` |
| Critiques de nuit (22/10) | 12 rapports par rôle et une synthèse | `docs/wip/nuit-critiques/SYNTHESE.md` |

## Limites connues
- **Performance** : la cible de 40 i/s en qualité ultra n'est pas atteinte (playtest du 17/10). La mesure
  du lot VQ a été faite sur un banc non silencieux ; à refaire sur un banc `--offscreen` reproductible.
- **Équilibrage** : écarts ouverts documentés (trésor de l'Empire, France dominante). Les révoltes sont
  sous la cible (≈ 2,7 par partie, mesures de 0 à 4,3 selon les bots, bruit élevé). Aucune ligne de base
  seed-bots n'est figée avant la refonte.
- **Sauvegardes** : la version est écrite mais jamais contrôlée au chargement, et il n'existe pas encore de
  migration (`core/crates/game-api/src/saves.rs`). Un changement de `CampaignState` rendrait les anciennes
  sauvegardes fragiles.
- **Intégration continue** : `client-bevy.yml` ne tourne que sur PR et à la demande ; les tests Python,
  `clippy` et `fmt` ne sont pas lancés en CI.
- **Données** : une partie des constantes de règles reste codée en dur dans `core/` (sièges, IA) ; les
  schémas `data/schemas/` ne sont pas vérifiés par le code Rust au chargement.
- **Interface** : barre du haut sans libellés, guide du tutoriel qui masque la carte, alertes non
  hiérarchisées (critique UI/UX du 22/10).
- **Animation et effets** : pas de mottes sous les sabots, pas d'étincelles d'impact, pas de test de
  couverture des clips (`docs/animation.md`, § « Ce qui manque encore »).
- **Documentation** : `docs/manuel.md` § 16 (Crédits) cite encore le moteur Godot.
- **Tests** : les chiffres du 09/10 (310 tests Rust, 87 tests Python) n'ont pas été recomptés.
- Les limites des jalons M1-M10 (factions de remplacement, mariages de l'IA, sièges sans sortie de
  garnison, etc.) sont dans `docs/archive/status-historique.md`.

## Commandes
- Lancer le jeu : `tools/launch.sh` (essai à blanc : `tools/launch.sh --dry-run`).
- Tests : `cd core && cargo test --workspace` ; `cd client && cargo test --features dev` ;
  `CENT_ANS_MUTE=1 client/tests/smoke.sh` ; `uv run --project tools pytest`.
- Budget : `uv run --project tools cent-ans budget summary`.
- Seed bots : `cd core && cargo run --profile seed-bots -p seed-bots -- run --quick` (ADR 0348).
- Règles de travail et commandes complètes : `CLAUDE.md`, `client/README.md`.
