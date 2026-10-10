# WR — restes du chantier WH (Warhammer III)

Demande joueur (2026-10-10) : faire tous les restes de `docs/wip/wh-warhammer3.md` (« Restes à faire »), en
autonomie complète (choix de conception, revue, fusion dans main, push), 0 $.
Décision joueur : **exécuter un captif** = prestige + pour le bourreau, rançon perdue, forte baisse d'opinion
de la faction du captif, légère baisse auprès des autres seigneurs (déshonneur chevaleresque).

Sessions parallèles : UX5 (`../gp-ux5-*`, Godot seul : écrans diplomatie, savoirs, recrutement, Colonies, Unités),
CO (`docs/wip/co-colonies.md`, colonies/emplacements/onglet Bâtiments, ADR 0291-0293). Ne pas toucher leurs fichiers
au-delà du nécessaire. **ADR WR : 0300-0309.**

Hors chantier : plafond d'emplacements (repris par CO : 6/4/3/3/2) ; ligue contre l'hégémon (arbitrage WH écrit :
seuils gardés, filet de sécurité).

Consignes communes des agents : `docs/wip/wh/brief-lot.md` (worktree `../gp-wr-<lot>`, branche `wr/<lot>`).

## Lots
| Lot | Contenu | ADR | État |
|---|---|---|---|
| captives | exécution des captifs (ordre, effets data, bouton « Exécuter » à deux clics) ; IA 15 % | 0303 | FUSIONNÉ |
| ai-agents | IA : assassinat, poison, guider une armée, embuscade d'espion (`agents/ai_strikes.rs`) ; ~3 attentats/partie | 0300 | FUSIONNÉ |
| turn | offre de 3 missions au choix (expire en 2 tours) ; filtre du journal par genre | 0304 | FUSIONNÉ |
| loyalty | rançon refusée −4/saison, titre à un pair −8 (+4 ambitieux) ; loyauté initiale ~76 ; 12 icônes de rôle | 0307 | FUSIONNÉ |
| ai-diplo | IA : JoinWar rare plafonné, non-agression (~85 paires) ; seuil d'impôt provincial 101 → 70 | 0302 | FUSIONNÉ |
| armies | renforts : pleins ≤ 10 km puis 40 % à 25 km, aperçu « à N km, ~X % » ; `SendCharacter` (150 km/tour), IA ≤ 4 envois/tour | 0305 | FUSIONNÉ |
| ai-mil | IA : RecruitInto, sortie, sommation ; embuscade castillane corrigée (`ambush_orders_after`) ; FR-EN en guerre ~57 % | 0301 | FUSIONNÉ |
| sortie | sortie jouée en bataille (terrain de la province, sans murs), choix dans le PreBattleDialog | 0306 | FUSIONNÉ |
| oeil | contrôle visuel (`game/tests/wr_shot.gd`, 5 captures) : offre de missions, captifs (« Confirmer l'exécution ? »), dialogue de sortie OK ; barre de genres du journal coupée → passée en `HFlowContainer` | — | FAIT |

## Restes (à reprendre en session future)
- **Exécution en fin de bataille** : choisir d'exécuter un captif depuis l'écran de fin de bataille ; aujourd'hui
  seulement depuis le panneau des rançons (`ransom_panel.gd`, ordre `ExecuteCaptive`, ADR 0303).
- **Agents IA immobiles** : `agents/ai_strikes.rs` frappe seulement les cibles déjà à portée ; les agents ne se
  déplacent pas vers elles (ADR 0300). Pas non plus de cible « héritier ».
- **UI de `SendCharacter`** : l'envoi d'un personnage vers une place ou une armée existe dans le cœur et l'IA
  (ADR 0305) mais pas d'interface joueur. L'arrivée datée des renforts n'est pas non plus montrée en bataille 3D.
- **Batailles +18 %** sur 6 graines depuis le lot `armies` (renforts selon la distance, ADR 0305) : à surveiller
  en partie pilote.

## Clôture (2026-10-10)
Tout est dans main et poussé. Non vérifiés à l'œil : aperçu « à N km, ~X % » des renforts, pastilles et chevrons,
bouton de sommation (couverts par les tests). Reprendre les restes ci-dessus dans une session future.
