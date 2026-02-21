pub const ITEMS_QUERY: &str = r#"
query SearchItems($name: String!, $lang: LanguageCode) {
    items(name: $name, lang: $lang) {
        id name shortName basePrice weight width height
        description gridImageLink iconLink
        avg24hPrice low24hPrice high24hPrice
        sellFor { vendor { name } price currency }
        categories { name }
    }
}
"#;

pub const AMMO_QUERY: &str = r#"
query SearchAmmo($lang: LanguageCode) {
    ammo(lang: $lang) {
        item { id name shortName iconLink gridImageLink }
        caliber damage armorDamage penetrationPower
        penetrationChance ricochetChance fragmentationChance
        projectileCount
    }
}
"#;

pub const TRADERS_QUERY: &str = r#"
query SearchTraders($lang: LanguageCode) {
    traders(lang: $lang) {
        id name description imageLink resetTime
        levels { level requiredPlayerLevel requiredReputation requiredCommerce }
        currency { name }
    }
}
"#;

// Note: startRewards is intentionally omitted. The Task struct has no start_rewards field.
// If start rewards are needed in the future, add both the query field and the struct field together.
// taskRequirements provides quest chain data for "퀘스트 체인 시각화" (issue #11).
pub const TASKS_QUERY: &str = r#"
query SearchTasks($lang: LanguageCode) {
    tasks(lang: $lang) {
        id name
        trader { name }
        map { name }
        minPlayerLevel
        objectives { description optional }
        finishRewards { items { item { name } count } }
        taskRequirements { task { id name } }
    }
}
"#;

// MAPS_QUERY includes boss spawn chances per map. The /boss command cross-references
// this data to show spawn locations (issue #13: "보스 스폰 위치").
pub const MAPS_QUERY: &str = r#"
query SearchMaps($lang: LanguageCode) {
    maps(lang: $lang) {
        id name description raidDuration players
        bosses { name spawnChance }
        extracts { name faction }
    }
}
"#;

// BOSSES_QUERY returns boss stats (health, equipment, drop items). Spawn location data is
// derived from MAPS_QUERY by filtering maps where the boss name appears.
// The /boss command executes both queries and merges the results.
// `equipment` = gear the boss wears; `items` = loot dropped when boss is killed (issue #13: "드롭 아이템").
pub const BOSSES_QUERY: &str = r#"
query SearchBosses($lang: LanguageCode) {
    bosses(lang: $lang) {
        name health { max }
        imagePosterLink
        equipment { item { name } }
        items { item { name } }
    }
}
"#;

pub const HIDEOUT_QUERY: &str = r#"
query SearchHideout($lang: LanguageCode) {
    hideoutStations(lang: $lang) {
        id name
        levels {
            level constructionTime
            itemRequirements { item { name } count }
            stationLevelRequirements { station { name } level }
        }
    }
}
"#;

pub const ALL_ITEMS_QUERY: &str = r#"
query AllItems($lang: LanguageCode) {
    items(lang: $lang) {
        id name shortName
    }
}
"#;

pub const CRAFTS_QUERY: &str = r#"
query SearchCrafts($lang: LanguageCode) {
    crafts(lang: $lang) {
        id duration
        station { name }
        level
        rewardItems { item { name avg24hPrice } count }
        requiredItems { item { name avg24hPrice } count }
    }
}
"#;
