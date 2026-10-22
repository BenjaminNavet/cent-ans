# 0723 — Sort de la ville prise : culture, remise, libération, banditisme

Statut : accepté (22/10, lot M2 `tx/m2-ville-prise`).

## Contexte
`capture.rs` (ADR 0101) donne quatre sorts à une place prise (occuper, rançon, piller, raser) avec des
troubles fixes, sans égard à la culture, sans banditisme et sans moyen de la rendre. Total War: Warhammer III
module les troubles selon la culture et offre `gift_to_another_faction` et `liberate`.

## Décision
- (a) `foreign_culture_extra` (par sort, `data/rules/capture.json`) s'ajoute aux troubles quand la culture
  de la province n'est pas celle du preneur : occuper +4, rançon +4, piller +6, raser +10. Il passe
  par `occupation_unrest` (occupation) et `preview` (autres sorts), donc par l'aperçu.
- (c) Banditisme : `banditry_devastation` / `banditry_turns` par sort (piller 10 / 4, raser 15 / 6).
  Dévastation temporaire (inscrite dans `CaptureState::banditry`), retirée à l'échéance. Pas de champ
  sur `ProvinceState` (sauvegardes inchangées).
- (b) Deux sorts de plus, `Gift` (« Remettre ») et `Liberate` (« Libérer »), avec un bénéficiaire
  (`Order::ChooseCaptureOutcome.recipient`, défaut serde `None`, l'aperçu propose le défaut).
  - Remettre : à un allié ou vassal vivant, en paix avec le preneur (propriétaire et contrôleur changent) ;
    troubles d'occupation annulés ; opinion du bénéficiaire +20 pendant 20 tours.
  - Libérer : au propriétaire de droit (`owner`) s'il n'est ni le preneur ni l'ancien tenant, vivant et en
    paix avec lui ; troubles annulés ; opinion de l'ancien tenant +30, de ses alliés +10, prestige +5 ;
    **les rancunes M1 sur cette place tombent** (`grudges::forget_settlement`). Pas de « prétendant »
    (hors périmètre, pas de notion de prétendant à la place).
  - Abandonner reste un non-choix. vassal / treasure_hunt / colonise ignorés.
- IA (`capture::ai_choice`) : scores `gift` / `liberate` ajoutés aux tables, avec `allied_present_bonus`
  (armée alliée sur la place) et `liberate_bonus` (tenant de droit en paix) ; sans tirage.

## Conséquences
- Les anciens tests à quatre options passent à six. Sondes de révoltes à surveiller (troubles +4 à +10 sur
  culture étrangère).
