# 0456 — Historique client dans la sauvegarde

## Contexte
Après sauvegarde puis chargement, `CampaignState` est identique mais le journal (70 lignes), la
chronique (lettres), le bloc « Monde » et les événements féodaux récents disparaissent (rapport
playtest 17/10, défaut 14). Ces historiques sont construits par le client tour après tour : le
cœur ne garde que les événements du dernier tour. De plus, `OnEnter(Campaign)` remettait ces
ressources à zéro à chaque rentrée sur l'écran de campagne (chargement, retour de bataille).

## Décision
- Le wrapper de sauvegarde (`game_api::saves::SaveWrapper`) gagne un champ optionnel `client`
  (JSON opaque, absent s'il est nul : les anciennes sauvegardes se chargent avec un historique vide).
  `CampaignState` et `STATE_VERSION` ne changent pas : le cœur ignore ce champ.
- Le client (`menu/save_extras.rs`) y écrit le journal (`ChronicleLog`) et la liste féodale
  récente ; un miroir process-wide (comme `flow::history`) alimente `save_now`, qui n'a pas le
  monde ECS. Au chargement, l'historique est restitué à l'entrée de la campagne ; un retour de
  bataille le conserve ; une nouvelle partie le vide.
- Tutoriel : la progression est une préférence du joueur, déjà dans les réglages
  (`tutorial_step/done/postponed`), pas dans la sauvegarde. Les répliques entendues du conseiller
  restent dans `advisor_seen.json`. La réplique d'ouverture du conseiller ne se joue qu'au tour 0
  (ni après un chargement, ni après une bataille).

## Conséquences
Les sauvegardes restent lisibles par les versions sans le champ (il est ignoré). Un nouveau
historique client se range dans `ClientExtras` avec `#[serde(default)]`.
