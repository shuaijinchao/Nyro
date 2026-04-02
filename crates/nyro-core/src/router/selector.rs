use crate::db::models::{RouteStrategy, RouteTarget};
use rand::Rng;
use std::collections::BTreeMap;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct SelectedTarget {
    pub provider_id: String,
    pub model: String,
}

pub struct TargetSelector;

impl TargetSelector {
    pub fn select_ordered(strategy: &str, targets: &[RouteTarget]) -> Vec<SelectedTarget> {
        let parsed = RouteStrategy::from_str(strategy).unwrap_or_default();
        match parsed {
            RouteStrategy::Weighted => weighted_select(targets),
            RouteStrategy::Priority => priority_select(targets),
        }
    }
}

fn weighted_select(targets: &[RouteTarget]) -> Vec<SelectedTarget> {
    let refs: Vec<&RouteTarget> = targets.iter().filter(|target| target.weight > 0).collect();
    weighted_shuffle(&refs)
        .into_iter()
        .map(|target| SelectedTarget {
            provider_id: target.provider_id.clone(),
            model: target.model.clone(),
        })
        .collect()
}

fn priority_select(targets: &[RouteTarget]) -> Vec<SelectedTarget> {
    let mut groups: BTreeMap<i32, Vec<&RouteTarget>> = BTreeMap::new();
    for target in targets {
        groups.entry(target.priority).or_default().push(target);
    }

    let mut ordered = Vec::new();
    for (_, group) in groups {
        for target in group {
            ordered.push(SelectedTarget {
                provider_id: target.provider_id.clone(),
                model: target.model.clone(),
            });
        }
    }
    ordered
}

fn weighted_shuffle<'a>(targets: &[&'a RouteTarget]) -> Vec<&'a RouteTarget> {
    if targets.is_empty() {
        return vec![];
    }
    let mut rng = rand::thread_rng();
    let mut items: Vec<(&RouteTarget, f64)> = targets
        .iter()
        .map(|target| {
            let weight = target.weight.max(1) as f64;
            let key = rng.r#gen::<f64>().powf(1.0 / weight);
            (*target, key)
        })
        .collect();
    items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    items.into_iter().map(|(target, _)| target).collect()
}
