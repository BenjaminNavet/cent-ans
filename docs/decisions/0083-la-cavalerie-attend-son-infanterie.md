# 0083 — La cavalerie de l'assaillant attend son infanterie sous les flèches

Date : 2026-09-26. Statut : accepté. Lot EQ7, suite de l'ADR 0052 (§ « Suites possibles »).

## Contexte

L'ADR 0052 (panique des chevaux sous les traits) a rendu aux positions anglaises leurs victoires
sur crête, au prix de la bataille mixte sans site (`b6`, `no_site_sim`). Dans cette bataille,
2 chevaliers, 2 hommes d'armes et 2 arbalètes français (5 800) affrontent 1 homme d'armes,
2 arcs longs et 1 régiment de chevaliers anglais (3 750). Les Français y sont passés de 51 à
30 victoires sur 64, puis à 31/64 après EP10-EP11.

La trace (graine 3) montre la cause. Dès que le contact passe sous 250 m (`view.assault`, lot
F5d), la cavalerie française part seule vers les chevaliers anglais (étape 1b de `plan_horse`),
alors que son infanterie est encore 250 m en arrière. Elle traverse les flèches, les chevaux
paniquent, et elle se débande avant l'arrivée de l'infanterie. Celle-ci finit la bataille seule.
C'est la faute de Crécy, pas sa leçon.

## Décision

- **Règle** (`waits_for_foot` dans `sim-battle/src/ai.rs`, réglages dans
  `data/rules/battle_horse_wait.json`, schéma `battle_horse_wait_rules.schema.json`, module
  `horse_wait.rs`). La règle s'applique à la cavalerie d'un **assaillant** qui a de l'infanterie.
  Cette cavalerie ne charge ni la cavalerie ennemie (étape 1b) ni un régiment ébranlé (étape 5)
  si la cible est couverte par des tireurs ennemis pourvus de traits. Une cible est couverte quand
  elle est à moins de portée effective × `range_margin` = 1,1 d'un tireur à pied qui a des traits
  et n'est pas pris en mêlée. L'interdiction dure tant que son infanterie n'est pas à moins de
  `foot_close_m` = **200 m** de cette cible, ni déjà en mêlée. En attendant, la cavalerie tient
  l'aile, qui avance avec la ligne. Une troupe déjà à moins de `committed_m` = 90 m de sa cible ne
  s'arrête plus : une charge arrêtée à l'arrêt sous les flèches serait la pire issue.
- **Ce qui ne change pas.** La contre-charge contre une cavalerie ennemie qui vient (étape 1) et
  la prise de flanc d'un régiment déjà en mêlée (étape 3) ne changent pas : l'infanterie est alors
  engagée, ou c'est l'ennemi qui vient. La charge sur des tireurs isolés (étape 2) ne change pas
  non plus : elle fixe ces tireurs et gêne leur tir (voir Mesures, EP9b).
- **Seulement l'assaillant.** Appliquée aussi au défenseur, la règle retenait sa cavalerie, qui
  ne sortait plus à la rencontre de la charge adverse. La bataille symétrique d'EP9b
  (`symmetric_flat_battle_is_open`, 3 à 7 victoires sur 10 attendues) passait alors à 10/10 pour
  l'assaillant. Le défenseur attend déjà l'ennemi (ADR 0046, R4).
- **Prise de flanc.** Elle n'a pas été retenue comme manœuvre distincte. Les tireurs du simulateur
  pivotent pour tirer dans toutes les directions : un détour hors de portée ne protège pas des
  flèches, il retarde seulement la charge. L'étape 3, qui prend de flanc un régiment déjà en mêlée,
  couvre le cas utile.

## Mesures

Main du 26/09 (7e1ac032) comparé à EQ7. Bataille mixte `b6`, graines 0-63 (sonde
`eq7_cavalry::probe_mixed_battle`) :

| `foot_close_m` | victoires françaises | tests de rythme B4 (contact 55-180 s) |
|---|---|---|
| main | 31/64 | — |
| 120 | 46/64 | 4 échecs (contact démo à 180 s, graine 2 du bocage à 255 s) |
| 160 | 50/64 | 1 échec (bocage graine 5 à 170 s) |
| **200 (retenu)** | **55/64**, puis **57/64** avec la règle limitée à l'assaillant | tous passent |
| 250 | 53/64 | tous passent |
| 300 | 30/64 (la règle ne joue plus) | tous passent |

À 200 m, la cavalerie part quand son infanterie est à une minute de marche de la cible. Elle arrive
un peu avant elle, sans avoir traversé seule les flèches. À 120 m, le premier contact de la
bataille de démonstration glissait à 3 minutes : c'était trop lent.

57/64 dépasse les 51/64 d'avant l'ADR 0052. L'armée française coûte 55 % de plus et combat en rase
campagne, sans site : qu'elle gagne neuf fois sur dix quand elle coordonne ses armes est voulu. La
force anglaise vient du terrain, et le terrain ne change pas.

Non-régression :

- R4, `survey_english_position_against_knights`, `R4_FRENCH=heavy R4_JITTER=1`, 32 graines,
  victoires anglaises : crête + haie 18, crête nue 28, haie en creux + crête 24, rase campagne 1,
  **identique** au main. Même résultat sans `R4_FRENCH` : 32/32 partout. La règle ne change aucune
  de ces batailles (cause non tracée).
- R2b, `survey_active_against_passive`, IA active contre passive, 128 batailles : 118 → **115**
  (plaine 31 → 29, collines 28 → 27, bocage et montagne inchangés). L'IA active attaque moins tôt
  avec sa cavalerie contre un camp passif pourvu d'archers.
- EP9b, `a_won_duel_holds_the_line_then_the_battle_ends`. Si la règle couvre aussi l'étape 2, la
  cavalerie de l'assaillant ne charge plus les arbalétriers isolés. Ceux-ci tirent alors librement,
  le duel n'est plus « gagné » au sens d'EP9b, et la ligne avance à 180 s comme si le duel était
  limité. D'où l'étape 2 laissée hors de la règle.
- Empreintes de `battles_without_a_site_are_unchanged` : les graines 3 et 11 passent aux Français
  (268 s et 245 s).
- Nouveau test `french_knights_wait_for_their_foot_and_win_the_mixed_battle` : graines 0-15, au
  moins 12 victoires françaises (15 mesurées ; le main en donne environ la moitié).

## Conséquences

- L'IA assaillante combine enfin ses armes contre des archers. Sa cavalerie arrive avec son
  infanterie, et les archers doivent partager leur tir.
- Le premier contact vient un peu plus tard quand la cavalerie l'ouvrait seule. Le rythme B4 est
  tenu.
- L'auto-résolution de campagne n'est pas touchée.
- Pour le joueur, rien ne change pour ses propres troupes. L'IA adverse devient plus difficile à
  battre avec une armée d'archers en rase campagne ; sur une position (crête, haie), rien ne change.
