use sqlx::SqlitePool;

use crate::db::models::Route;

pub struct RouteCache {
    pub routes: Vec<Route>,
}

impl RouteCache {
    pub async fn load(pool: &SqlitePool) -> anyhow::Result<Self> {
        let routes: Vec<Route> = sqlx::query_as::<_, Route>(
            r#"SELECT
                id, name, match_pattern, target_provider, target_model,
                fallback_provider, fallback_model,
                is_active,
                priority,
                created_at
            FROM routes
            WHERE is_active = 1
            ORDER BY priority ASC"#,
        )
        .fetch_all(pool)
        .await?;

        Ok(Self { routes })
    }

    pub async fn reload(&mut self, pool: &SqlitePool) -> anyhow::Result<()> {
        *self = Self::load(pool).await?;
        Ok(())
    }
}

pub fn match_route<'a>(routes: &'a [Route], model: &str) -> Option<&'a Route> {
    let mut best: Option<(u8, &'a Route)> = None;

    for route in routes {
        let score = match_score(&route.match_pattern, model);
        if score == 0 {
            continue;
        }
        if best.is_none() || score > best.unwrap().0 {
            best = Some((score, route));
        }
    }

    best.map(|(_, r)| r)
}

fn match_score(pattern: &str, model: &str) -> u8 {
    if pattern == model {
        return 3; // exact
    }
    if pattern != "*" && glob_match::glob_match(pattern, model) {
        return 2; // glob
    }
    if pattern == "*" {
        return 1; // wildcard
    }
    0
}
