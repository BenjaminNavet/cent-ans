# 0278 — Diplomatie lisible (lot WH `diploa`)

## Contexte
Le rapport critique WH (`docs/wip/wh/diplomatie.md`) relève que la diplomatie, riche en règles, reste opaque :
le joueur déclare la guerre sans savoir quels alliés de la cible viendront, lit « attitude +27 » sans étiquette ni
durée, ne voit pas les alliances d'une faction tierce, ne peut pas retirer un accès militaire (irrévocable) et
ne retrouve ni ruptures ni parjures dans l'historique des traités.

## Décision
- **Prévision d'appel aux armes.** `answers_call_to_arms` devient le cas « Joins » de `call_to_arms_forecast`
  (`Joins | Hesitates | Refuses(raison)`) : une seule décision, deux lectures (la guerre réelle et l'aperçu), donc
  jamais d'écart. L'aperçu (`evaluate_proposal` d'une déclaration) liste pour chaque allié (et suzerain) de la cible
  « viendra / hésite / refusera : raison » et le coût en prestige (`declaration_prestige_cost`, constantes
  `PERJURY_PRESTIGE` −30, `AGGRESSION_PRESTIGE` −20, déjà celles de `declare_war`). Seuil d'hésitation :
  `call_forecast.hesitate_floor` dans `data/rules/diplomacy.json`.
- **Étiquette d'attitude** : `attitude_bands` (data) lue par `DiplomacyRules::attitude_band`. Durée des
  modificateurs : `CampaignState::reason_turns_left` (« encore N tours », trêve comprise) exposée par motif.
- **Fiche de faction** : `DiplomacyEntry` porte `allies`, `enemies`, `vassals` (ordre d'identifiant stable) ;
  l'UI en fait des boutons plats qui ouvrent la fiche (signal `faction_requested`).
- **Révocation** : `Order::RevokeMilitaryAccess { target }` → `revoke_military_access` : retire l'accès, le bénéficiaire
  garde `revoke_access.attitude` (−15) pendant `revoke_access.duration` (20) tours (« Accès retiré »).
- **Historique** : `TreatyRecord.rupture: Option<Rupture>` (`Broken | Perjury | RefusedCall`, `#[serde(default)]`),
  écrit des deux côtés par `record_rupture` à `break_alliance`, `break_trade_agreement`, `revoke_military_access`,
  `declare_war` sur trêve, et au refus d'un appel aux armes. `accepted` reste faux pour ces entrées.

## Conséquences
- Sauvegardes anciennes lisibles (champs `serde(default)`). L'ordre `revoke_military_access` est joueur uniquement
  pour l'instant (l'IA ne retire jamais un accès).
- Les valeurs −40/−20/−30 de `declare_war` (opinion, prestige) restent en constantes Rust, non déplacées en data.
- Les ruptures entre deux IA sont aussi enregistrées (historique borné par `HISTORY_LENGTH`).
