# ADR 0116 — Terrains, climats, religions et mers de l'Est

## Contexte
Les nouvelles régions (steppe pontique, déserts d'Afrique et du Levant, monde orthodoxe et musulman)
n'ont pas d'équivalent dans les énumérations actuelles.

## Décision
- Terrains `steppe` (plaine sèche : mouvement de plaine, ravitaillement réduit, avantage cavalerie)
  et `desert` (attrition, ravitaillement très réduit) ; climats `arid` et `steppe`. Les champs de
  bataille réutilisent le sol de plaine avec une teinte propre tant qu'aucun décor dédié n'existe.
- Religions : `rel_orthodox` (Église orthodoxe, `other_faith` : dans le modèle `church` signifie catholique ; la proximité passe par le champ `kindred` → `rel_catholic`, relation « schismatiques »), `rel_armenian` (Église apostolique,
  `other_faith`), `rel_pagan` (Lituanie, Finnois de la Volga, `other_faith`) ; `rel_islam` désigne
  l'islam sunnite en général.
- Zones maritimes ajoutées (identifiants libres) : ionienne, Égée, Levant, mer Noire, Caspienne
  (fermée, sans liaison), golfe de Botnie, mer de Norvège, mer Blanche.
- Les armées de l'Est réutilisent les types d'unités existants en v1.

## Conséquences
Chantier ultérieur pour les unités de l'Est (archers montés, mamelouks) et les décors de bataille
de steppe et de désert.
