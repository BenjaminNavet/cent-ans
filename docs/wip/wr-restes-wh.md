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
| Lot | Contenu | ADR | Agent | État |
|---|---|---|---|---|
| ai-agents | IA : assassinat, poison, guider une armée, embuscade d'espion | 0300 | dev | vague 1 |
| ai-mil | IA : recruter dans une armée (RecruitInto), sortie, sommation | 0301 | dev | vague 1 |
| ai-diplo | IA : demande d'entrée en guerre (JoinWar), pacte de non-agression ; seuil d'impôt provincial réglé | 0302 | dev | vague 1 |
| captives | exécution des captifs (ordre, effets data, bouton UI) | 0303 | dev | FUSIONNÉ (358d587fa) ; reste : choix « exécuter » en fin de bataille |
| turn | mission proposée en choix (accepter/refuser parmi plusieurs) ; filtre par genre du journal | 0304 | dev | vague 1 |
| armies | renforts lointains affaiblis selon la distance ; personnages libres qui rejoignent seuls une place/armée | 0305 | dev | vague 1 |
| sortie | sortie jouée en bataille (comme un assaut, défenseur attaquant) | 0306 | dev | vague 2 |
| loyalty | baisse de loyauté (rançon refusée, titre donné à un rival) ; loyauté initiale non pleine ; icônes des 12 compétences de rôle | 0307 | mech | vague 2 |
| oeil | contrôle visuel (≤ 10 captures) : « à N km », impôt, pastilles, chevrons, sommation, exécution, missions | — | session | fin |

## Prochaine étape
Vague 1 lancée ; fusionner chaque lot (rebase, tests, ff-only), puis vague 2, puis contrôle visuel et push.
