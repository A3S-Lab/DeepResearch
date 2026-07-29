#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct DeepResearchSourceAttribution {
    source_group_ids: std::collections::BTreeMap<String, String>,
    independent_group_pairs: std::collections::BTreeSet<(String, String)>,
}

impl DeepResearchSourceAttribution {
    fn group_id(&self, source_alias: &str) -> Option<&str> {
        self.source_group_ids.get(source_alias).map(String::as_str)
    }

    fn has_verified_independent_pair<'a>(
        &self,
        source_aliases: impl IntoIterator<Item = &'a str>,
    ) -> bool {
        let groups = source_aliases
            .into_iter()
            .filter_map(|source_alias| self.group_id(source_alias))
            .collect::<HashSet<_>>();
        self.independent_group_pairs
            .iter()
            .any(|(left, right)| groups.contains(left.as_str()) && groups.contains(right.as_str()))
    }

    fn has_verified_independent_pair_between<'a, 'b>(
        &self,
        left_source_aliases: impl IntoIterator<Item = &'a str>,
        right_source_aliases: impl IntoIterator<Item = &'b str>,
    ) -> bool {
        let left_groups = left_source_aliases
            .into_iter()
            .filter_map(|source_alias| self.group_id(source_alias))
            .collect::<HashSet<_>>();
        let right_groups = right_source_aliases
            .into_iter()
            .filter_map(|source_alias| self.group_id(source_alias))
            .collect::<HashSet<_>>();
        self.independent_group_pairs.iter().any(|(left, right)| {
            (left_groups.contains(left.as_str()) && right_groups.contains(right.as_str()))
                || (left_groups.contains(right.as_str())
                    && right_groups.contains(left.as_str()))
        })
    }

    fn independently_attributable_group_count<'a>(
        &self,
        source_aliases: impl IntoIterator<Item = &'a str>,
    ) -> usize {
        let groups = source_aliases
            .into_iter()
            .filter_map(|source_alias| self.group_id(source_alias))
            .collect::<HashSet<_>>();
        self.independent_group_pairs
            .iter()
            .filter(|(left, right)| {
                groups.contains(left.as_str()) && groups.contains(right.as_str())
            })
            .flat_map(|(left, right)| [left.as_str(), right.as_str()])
            .collect::<HashSet<_>>()
            .len()
    }

    fn closed_packet(&self) -> serde_json::Value {
        let mut sources_by_group = std::collections::BTreeMap::<&str, Vec<&str>>::new();
        for (source_alias, group_id) in &self.source_group_ids {
            sources_by_group
                .entry(group_id.as_str())
                .or_default()
                .push(source_alias.as_str());
        }
        serde_json::json!({
            "version": 1,
            "groups": sources_by_group.into_iter().map(|(group_id, source_aliases)| {
                serde_json::json!({
                    "group_id": group_id,
                    "source_ids": source_aliases,
                })
            }).collect::<Vec<_>>(),
            "independent_group_pairs": self.independent_group_pairs.iter().map(|(left, right)| {
                serde_json::json!({"group_ids": [left, right]})
            }).collect::<Vec<_>>(),
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DeepResearchAttributedSourceCatalog {
    pub(crate) catalog: DeepResearchSourceCatalog,
    pub(crate) attribution: DeepResearchSourceAttribution,
}

fn catalog_source_attribution(
    value: Option<&serde_json::Value>,
    raw_sources: &[serde_json::Value],
    catalog_index_by_source_id: &HashMap<String, usize>,
    catalog: &DeepResearchSourceCatalog,
) -> DeepResearchSourceAttribution {
    fn project(
        value: Option<&serde_json::Value>,
        raw_sources: &[serde_json::Value],
        catalog_index_by_source_id: &HashMap<String, usize>,
        catalog: &DeepResearchSourceCatalog,
    ) -> Option<DeepResearchSourceAttribution> {
        let contract = value?.as_object()?;
        if contract.len() != 3
            || contract.get("version").and_then(serde_json::Value::as_u64) != Some(1)
        {
            return None;
        }
        let raw_groups = contract.get("groups")?.as_array()?;
        let raw_pairs = contract.get("independent_group_pairs")?.as_array()?;
        if raw_groups.is_empty()
            || raw_groups.len() > raw_sources.len()
            || raw_pairs.len()
                > raw_groups
                    .len()
                    .saturating_mul(raw_groups.len().saturating_sub(1))
                    / 2
        {
            return None;
        }

        let mut expected_source_ids = HashSet::<String>::new();
        for raw_source in raw_sources {
            let source_id = bounded_catalog_text(
                raw_source.get("source_id"),
                160,
                stable_catalog_identity,
            )?;
            if !expected_source_ids.insert(source_id.clone())
                || !catalog_index_by_source_id.contains_key(&source_id)
            {
                return None;
            }
        }
        if expected_source_ids.len() != catalog_index_by_source_id.len() {
            return None;
        }

        let mut seen_source_ids = HashSet::<String>::new();
        let mut group_index_by_id = HashMap::<String, usize>::new();
        let mut projected_raw_groups = Vec::<std::collections::BTreeSet<usize>>::new();
        for (group_index, raw_group) in raw_groups.iter().enumerate() {
            let group = raw_group.as_object()?;
            if group.len() != 2 {
                return None;
            }
            let group_id = group
                .get("group_id")?
                .as_str()
                .map(str::trim)
                .filter(|group_id| stable_catalog_identity(group_id))?;
            if group_index_by_id
                .insert(group_id.to_string(), group_index)
                .is_some()
            {
                return None;
            }
            let source_ids = group
                .get("source_ids")?
                .as_array()
                .filter(|source_ids| {
                    !source_ids.is_empty() && source_ids.len() <= expected_source_ids.len()
                })?;
            let mut projected = std::collections::BTreeSet::new();
            for source_id in source_ids {
                let source_id = source_id.as_str()?;
                if !expected_source_ids.contains(source_id)
                    || !seen_source_ids.insert(source_id.to_string())
                {
                    return None;
                }
                projected.insert(*catalog_index_by_source_id.get(source_id)?);
            }
            if projected.is_empty() {
                return None;
            }
            projected_raw_groups.push(projected);
        }
        if seen_source_ids != expected_source_ids {
            return None;
        }

        let mut raw_independent_pairs = std::collections::BTreeSet::<(usize, usize)>::new();
        for raw_pair in raw_pairs {
            let pair = raw_pair.as_object()?;
            if pair.len() != 1 {
                return None;
            }
            let group_ids = pair
                .get("group_ids")?
                .as_array()
                .filter(|group_ids| group_ids.len() == 2)?;
            let left = *group_index_by_id.get(group_ids[0].as_str()?)?;
            let right = *group_index_by_id.get(group_ids[1].as_str()?)?;
            if left == right {
                return None;
            }
            let canonical = if left < right {
                (left, right)
            } else {
                (right, left)
            };
            if !raw_independent_pairs.insert(canonical) {
                return None;
            }
        }

        // Canonical-anchor coalescing can project several raw source IDs onto
        // one report alias. Close attribution groups transitively over every
        // shared alias before retaining any positive independence edge.
        let mut components = Vec::<std::collections::BTreeSet<usize>>::new();
        for raw_group in &projected_raw_groups {
            let mut component = raw_group.clone();
            let mut index = 0usize;
            while index < components.len() {
                if component.is_disjoint(&components[index]) {
                    index += 1;
                } else {
                    let overlapping = components.remove(index);
                    component.extend(overlapping);
                }
            }
            components.push(component);
        }
        components.sort_by_key(|component| component.first().copied().unwrap_or(usize::MAX));
        let projected_indexes = components
            .iter()
            .flat_map(|component| component.iter().copied())
            .collect::<Vec<_>>();
        if projected_indexes.len() != catalog.sources.len()
            || projected_indexes
                .iter()
                .copied()
                .collect::<HashSet<_>>()
                .len()
                != catalog.sources.len()
        {
            return None;
        }

        let mut component_by_raw_group = Vec::with_capacity(projected_raw_groups.len());
        for raw_group in &projected_raw_groups {
            let component_index = components.iter().position(|component| {
                raw_group
                    .iter()
                    .all(|source_index| component.contains(source_index))
            })?;
            component_by_raw_group.push(component_index);
        }
        let canonical_group_ids = (0..components.len())
            .map(|index| format!("attribution-group-{}", index + 1))
            .collect::<Vec<_>>();
        let mut source_group_ids = std::collections::BTreeMap::new();
        for (component_index, component) in components.iter().enumerate() {
            for source_index in component {
                let source_alias = catalog.sources.get(*source_index)?.alias.clone();
                if source_group_ids
                    .insert(source_alias, canonical_group_ids[component_index].clone())
                    .is_some()
                {
                    return None;
                }
            }
        }
        let independent_group_pairs = raw_independent_pairs
            .into_iter()
            .filter_map(|(left, right)| {
                let left = component_by_raw_group[left];
                let right = component_by_raw_group[right];
                if left == right {
                    return None;
                }
                let left = canonical_group_ids[left].clone();
                let right = canonical_group_ids[right].clone();
                Some(if left < right {
                    (left, right)
                } else {
                    (right, left)
                })
            })
            .collect::<std::collections::BTreeSet<_>>();
        Some(DeepResearchSourceAttribution {
            source_group_ids,
            independent_group_pairs,
        })
    }

    project(value, raw_sources, catalog_index_by_source_id, catalog).unwrap_or_default()
}
