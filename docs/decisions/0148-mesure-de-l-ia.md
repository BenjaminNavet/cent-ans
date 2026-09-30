# 0148 — Mesurer l'IA avant de la changer : sondage de bataille et duel A/B de campagne

Date : 2026-10-01 (nuit IA, `docs/wip/ia-nuit.md`).

## Contexte

Les réglages de l'IA (bataille : `sim-battle/src/ai.rs` ; campagne : `ai/src/campaign.rs`,
`data/ai/*.json`) ont été ajustés lot après lot sur des scénarios isolés. En bataille, la seule
référence était un camp passif. En campagne, les sondes globales (`century_probe`,
`balance_probe`, `ia_quality_probe`) mesurent l'équilibre et des symptômes, mais d'une graine à
l'autre la partie diverge tant qu'un écart de 20 % sur un indicateur reste du bruit : cette nuit,
deux règles de bon sens (secourir d'abord une place assiégée, ne pas assiéger sous une armée plus
forte) semblaient aider sur les indicateurs et se sont révélées nuisibles.

## Décision

1. **Bataille** : toute règle d'IA se juge sur `tests/ia_survey.rs` (ignoré) : 16 graines ×
   4 reliefs × 2 camps contre trois références, à savoir un novice (chaque régiment attaque
   l'ennemi le plus proche), un camp passif et l'IA elle-même ; armées miroir, anglaise et
   française. Une règle est gardée si elle gagne au total sans perte nette sur l'armée miroir.
   Les empreintes (B6, bornes de rejeu) sont ensuite mises à jour avec la raison.
2. **Campagne** : toute règle se juge en duel (`ai::experiment`, `examples/ai_duel_probe.rs`).
   Une faction joue la règle, toutes les autres les règles en vigueur, et son sort (places
   pondérées, provinces, puissance, trésor) se compare à la même partie sans essai.
   Référence : 8 factions × 4 graines × 60 tours. Une règle est gardée si la faction qui la
   joue fait nettement mieux (mieux / pire et delta moyen des places).
3. `ai::experiment` reste dans le jeu comme outil de développement : personne ne joue d'essai
   par défaut, donc la partie n'est pas affectée. Une règle adoptée perd sa porte d'essai.

## Conséquences

- Bataille : la cavalerie de l'attaquant sous les flèches attend derrière son infanterie
  (92ec4dc18). Contre le novice et le camp passif, l'IA passe de 618 à 670 victoires sur 768.
- Campagne : aucune des 8 règles essayées n'a gagné son duel. Les réglages actuels tiennent
  face aux variantes plus prudentes comme plus agressives, et aucune règle de campagne ne change.
- Le coût d'un duel (≈ 1 h sur une machine chargée) réserve l'outil aux changements de règle,
  pas aux retouches de chiffres.
