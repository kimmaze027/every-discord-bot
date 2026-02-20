use serde::Deserialize;

// GraphQL response wrapper
#[derive(Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize, Debug)]
pub struct GraphQLError {
    pub message: String,
}

// Item-related
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: String,
    pub name: String,
    pub short_name: String,
    pub base_price: i64,
    pub weight: f64,
    pub width: i32,
    pub height: i32,
    pub description: Option<String>,
    pub grid_image_link: Option<String>,
    pub icon_link: Option<String>,
    pub avg24h_price: Option<i64>,
    pub low24h_price: Option<i64>,
    pub high24h_price: Option<i64>,
    pub sell_for: Vec<VendorPrice>,
    pub categories: Vec<Category>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct VendorPrice {
    pub vendor: Vendor,
    pub price: i64,
    pub currency: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Vendor {
    pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Category {
    pub name: String,
}

// Ammo
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Ammo {
    pub item: AmmoItem,
    pub caliber: String,
    pub damage: i32,
    pub armor_damage: i32,
    pub penetration_power: i32,
    pub penetration_chance: f64,
    pub ricochet_chance: f64,
    pub fragmentation_chance: f64,
    pub projectile_count: Option<i32>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AmmoItem {
    pub id: String,
    pub name: String,
    pub short_name: String,
    pub icon_link: Option<String>,
    pub grid_image_link: Option<String>,
}

// Trader
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Trader {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub image_link: Option<String>,
    pub reset_time: Option<String>,
    pub levels: Vec<TraderLevel>,
    pub currency: Currency,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct TraderLevel {
    pub level: i32,
    pub required_player_level: i32,
    pub required_reputation: f64,
    pub required_commerce: i64,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Currency {
    pub name: String,
}

// Task (Quest)
// Note: taskRequirements provides quest chain data (prerequisite quests).
// The /quest command displays these as "이전 퀘스트" (previous quests) for chain visualization.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub name: String,
    pub trader: TaskTrader,
    pub map: Option<TaskMap>,
    pub min_player_level: Option<i32>,
    pub objectives: Vec<TaskObjective>,
    // Note: start_rewards intentionally omitted; TASKS_QUERY only queries finishRewards.
    // Add startRewards to TASKS_QUERY if start_rewards is re-introduced here.
    pub finish_rewards: Option<TaskRewards>,
    pub task_requirements: Vec<TaskRequirement>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TaskTrader {
    pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TaskMap {
    pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TaskObjective {
    pub description: String,
    pub optional: bool,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TaskRewards {
    pub items: Vec<TaskRewardItem>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TaskRewardItem {
    pub item: RewardItemInfo,
    pub count: i32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RewardItemInfo {
    pub name: String,
}

// TaskRequirement: represents a prerequisite quest in the quest chain.
// Used by /quest to display "이전 퀘스트" (previous quests) for chain visualization (issue #11).
#[derive(Deserialize, Clone, Debug)]
pub struct TaskRequirement {
    pub task: TaskRequirementRef,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TaskRequirementRef {
    pub id: String,
    pub name: String,
}

// Map
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct GameMap {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub raid_duration: Option<i32>,
    pub players: Option<String>,
    pub bosses: Vec<MapBoss>,
    pub extracts: Vec<MapExtract>,
}

// MapBoss: spawn_chance is the boss's probability on this specific map.
// Boss spawn locations are shown via /map (which lists bosses per map).
// The /boss command cross-references map data for spawn location context.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MapBoss {
    pub name: String,
    pub spawn_chance: f64,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct MapExtract {
    pub name: String,
    pub faction: Option<String>,
}

// Boss
// Note: Boss spawn locations are derived by cross-referencing the maps query.
// BOSSES_QUERY returns boss stats (health, equipment, drop items); MAPS_QUERY provides per-map boss spawn chances.
// The /boss command queries both and lists maps where that boss appears.
// `equipment` = what the boss wears; `items` = what the boss drops when killed (issue #13: "드롭 아이템").
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Boss {
    pub name: String,
    pub health: Option<Vec<BossHealth>>,
    pub image_poster_link: Option<String>,
    pub equipment: Vec<BossEquipment>,
    pub items: Vec<BossLootItem>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BossHealth {
    pub max: i32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct BossEquipment {
    pub item: EquipmentItem,
}

#[derive(Deserialize, Clone, Debug)]
pub struct EquipmentItem {
    pub name: String,
}

// BossLootItem: items dropped by the boss when killed.
// Queried via BOSSES_QUERY `items { item { name } }` field.
#[derive(Deserialize, Clone, Debug)]
pub struct BossLootItem {
    pub item: LootItemInfo,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LootItemInfo {
    pub name: String,
}

// Hideout
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HideoutStation {
    pub id: String,
    pub name: String,
    pub levels: Vec<HideoutLevel>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct HideoutLevel {
    pub level: i32,
    pub construction_time: i32,
    pub item_requirements: Vec<HideoutItemReq>,
    pub station_level_requirements: Vec<StationLevelReq>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct HideoutItemReq {
    pub item: RewardItemInfo,
    pub count: i32,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct StationLevelReq {
    pub station: StationRef,
    pub level: i32,
}

#[derive(Deserialize, Clone, Debug)]
pub struct StationRef {
    pub name: String,
}

// Craft
// IMPORTANT: All fields are snake_case. serde(rename_all = "camelCase") maps:
//   reward_items  -> JSON "rewardItems"
//   required_items -> JSON "requiredItems"
// Never use camelCase field names in Rust structs; clippy will reject them.
#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Craft {
    pub id: String,
    pub duration: i32,
    pub station: CraftStation,
    pub level: i32,
    pub reward_items: Vec<CraftItem>,
    pub required_items: Vec<CraftItem>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CraftStation {
    pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CraftItem {
    pub item: CraftItemInfo,
    pub count: i32,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CraftItemInfo {
    pub name: String,
    pub avg24h_price: Option<i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_item() {
        let json = r#"{
            "id": "abc123",
            "name": "LEDX",
            "shortName": "LEDX",
            "basePrice": 100000,
            "weight": 0.5,
            "width": 1,
            "height": 1,
            "description": "의료 장비",
            "gridImageLink": "https://example.com/img.png",
            "iconLink": "https://example.com/icon.png",
            "avg24hPrice": 1500000,
            "low24hPrice": 1200000,
            "high24hPrice": 1800000,
            "sellFor": [
                {
                    "vendor": { "name": "치료사" },
                    "price": 300000,
                    "currency": "RUB"
                }
            ],
            "categories": [{ "name": "의료" }]
        }"#;
        let item: Item = serde_json::from_str(json).unwrap();
        assert_eq!(item.id, "abc123");
        assert_eq!(item.name, "LEDX");
        assert_eq!(item.short_name, "LEDX");
        assert_eq!(item.base_price, 100000);
        assert!((item.weight - 0.5).abs() < f64::EPSILON);
        assert_eq!(item.avg24h_price, Some(1500000));
        assert_eq!(item.sell_for.len(), 1);
        assert_eq!(item.sell_for[0].vendor.name, "치료사");
        assert_eq!(item.categories[0].name, "의료");
    }

    #[test]
    fn test_deserialize_ammo() {
        let json = r#"{
            "item": {
                "id": "ammo1",
                "name": "7.62x39mm BP",
                "shortName": "BP",
                "iconLink": null,
                "gridImageLink": null
            },
            "caliber": "7.62x39mm",
            "damage": 58,
            "armorDamage": 47,
            "penetrationPower": 47,
            "penetrationChance": 0.93,
            "ricochetChance": 0.05,
            "fragmentationChance": 0.12,
            "projectileCount": null
        }"#;
        let ammo: Ammo = serde_json::from_str(json).unwrap();
        assert_eq!(ammo.item.name, "7.62x39mm BP");
        assert_eq!(ammo.caliber, "7.62x39mm");
        assert_eq!(ammo.damage, 58);
        assert_eq!(ammo.penetration_power, 47);
        assert!(ammo.projectile_count.is_none());
    }

    #[test]
    fn test_deserialize_trader() {
        let json = r#"{
            "id": "trader1",
            "name": "프라포르",
            "description": "무기 거래상",
            "imageLink": "https://example.com/prapor.png",
            "resetTime": "2026-01-01T00:00:00Z",
            "levels": [
                {
                    "level": 1,
                    "requiredPlayerLevel": 1,
                    "requiredReputation": 0.0,
                    "requiredCommerce": 0
                },
                {
                    "level": 2,
                    "requiredPlayerLevel": 15,
                    "requiredReputation": 0.2,
                    "requiredCommerce": 1000000
                }
            ],
            "currency": { "name": "RUB" }
        }"#;
        let trader: Trader = serde_json::from_str(json).unwrap();
        assert_eq!(trader.name, "프라포르");
        assert_eq!(trader.levels.len(), 2);
        assert_eq!(trader.levels[1].required_player_level, 15);
        assert!((trader.levels[1].required_reputation - 0.2).abs() < f64::EPSILON);
        assert_eq!(trader.currency.name, "RUB");
    }

    #[test]
    fn test_deserialize_task() {
        let json = r#"{
            "id": "task1",
            "name": "보급품 조달",
            "trader": { "name": "프라포르" },
            "map": { "name": "세관" },
            "minPlayerLevel": 5,
            "objectives": [
                { "description": "아이템 3개 찾기", "optional": false },
                { "description": "보너스 목표", "optional": true }
            ],
            "finishRewards": {
                "items": [
                    { "item": { "name": "루블" }, "count": 50000 }
                ]
            },
            "taskRequirements": [
                { "task": { "id": "task0", "name": "첫 번째 퀘스트" } }
            ]
        }"#;
        let task: Task = serde_json::from_str(json).unwrap();
        assert_eq!(task.name, "보급품 조달");
        assert_eq!(task.trader.name, "프라포르");
        assert_eq!(task.map.as_ref().unwrap().name, "세관");
        assert_eq!(task.min_player_level, Some(5));
        assert_eq!(task.objectives.len(), 2);
        assert!(!task.objectives[0].optional);
        assert!(task.objectives[1].optional);
        let rewards = task.finish_rewards.as_ref().unwrap();
        assert_eq!(rewards.items[0].count, 50000);
        assert_eq!(task.task_requirements[0].task.name, "첫 번째 퀘스트");
    }

    #[test]
    fn test_deserialize_game_map() {
        let json = r#"{
            "id": "map1",
            "name": "세관",
            "description": "위험한 지역",
            "raidDuration": 45,
            "players": "8-12",
            "bosses": [
                { "name": "레쉴라", "spawnChance": 0.38 }
            ],
            "extracts": [
                { "name": "출구 A", "faction": "pmc" },
                { "name": "출구 B", "faction": null }
            ]
        }"#;
        let map: GameMap = serde_json::from_str(json).unwrap();
        assert_eq!(map.name, "세관");
        assert_eq!(map.raid_duration, Some(45));
        assert_eq!(map.bosses[0].name, "레쉴라");
        assert!((map.bosses[0].spawn_chance - 0.38).abs() < f64::EPSILON);
        assert_eq!(map.extracts.len(), 2);
        assert_eq!(map.extracts[0].faction, Some("pmc".to_string()));
        assert!(map.extracts[1].faction.is_none());
    }

    #[test]
    fn test_deserialize_boss() {
        let json = r#"{
            "name": "킬라",
            "health": [{ "max": 890 }, { "max": 70 }],
            "imagePosterLink": "https://example.com/killa.png",
            "equipment": [
                { "item": { "name": "RPK-16" } },
                { "item": { "name": "Maska-1Sch" } }
            ],
            "items": [
                { "item": { "name": "킬라의 헬멧" } }
            ]
        }"#;
        let boss: Boss = serde_json::from_str(json).unwrap();
        assert_eq!(boss.name, "킬라");
        let health = boss.health.as_ref().unwrap();
        assert_eq!(health[0].max, 890);
        assert_eq!(boss.equipment.len(), 2);
        assert_eq!(boss.equipment[0].item.name, "RPK-16");
        assert_eq!(boss.items.len(), 1);
        assert_eq!(boss.items[0].item.name, "킬라의 헬멧");
    }

    #[test]
    fn test_deserialize_hideout_station() {
        let json = r#"{
            "id": "station1",
            "name": "작업대",
            "levels": [
                {
                    "level": 1,
                    "constructionTime": 3600,
                    "itemRequirements": [
                        { "item": { "name": "볼트" }, "count": 3 }
                    ],
                    "stationLevelRequirements": [
                        { "station": { "name": "발전기" }, "level": 1 }
                    ]
                }
            ]
        }"#;
        let station: HideoutStation = serde_json::from_str(json).unwrap();
        assert_eq!(station.name, "작업대");
        assert_eq!(station.levels[0].construction_time, 3600);
        assert_eq!(station.levels[0].item_requirements[0].count, 3);
        assert_eq!(
            station.levels[0].station_level_requirements[0].station.name,
            "발전기"
        );
    }

    #[test]
    fn test_deserialize_craft() {
        let json = r#"{
            "id": "craft1",
            "duration": 7200,
            "station": { "name": "작업대" },
            "level": 2,
            "rewardItems": [
                { "item": { "name": "총기 부품", "avg24hPrice": 50000 }, "count": 1 }
            ],
            "requiredItems": [
                { "item": { "name": "볼트", "avg24hPrice": 10000 }, "count": 3 },
                { "item": { "name": "너트", "avg24hPrice": null }, "count": 5 }
            ]
        }"#;
        let craft: Craft = serde_json::from_str(json).unwrap();
        assert_eq!(craft.duration, 7200);
        assert_eq!(craft.station.name, "작업대");
        assert_eq!(craft.level, 2);
        assert_eq!(craft.reward_items[0].item.name, "총기 부품");
        assert_eq!(craft.reward_items[0].item.avg24h_price, Some(50000));
        assert_eq!(craft.required_items.len(), 2);
        assert!(craft.required_items[1].item.avg24h_price.is_none());
    }

    #[test]
    fn test_deserialize_graphql_response_with_data() {
        #[derive(Deserialize)]
        struct ItemsData {
            items: Vec<Item>,
        }
        let json = r#"{
            "data": {
                "items": [{
                    "id": "x",
                    "name": "Test",
                    "shortName": "T",
                    "basePrice": 100,
                    "weight": 0.1,
                    "width": 1,
                    "height": 1,
                    "description": null,
                    "gridImageLink": null,
                    "iconLink": null,
                    "avg24hPrice": null,
                    "low24hPrice": null,
                    "high24hPrice": null,
                    "sellFor": [],
                    "categories": []
                }]
            }
        }"#;
        let resp: GraphQLResponse<ItemsData> = serde_json::from_str(json).unwrap();
        assert!(resp.data.is_some());
        assert!(resp.errors.is_none());
        assert_eq!(resp.data.unwrap().items[0].name, "Test");
    }

    #[test]
    fn test_deserialize_graphql_response_with_errors() {
        #[derive(Deserialize)]
        struct Empty {}
        let json = r#"{
            "data": null,
            "errors": [{ "message": "Something went wrong" }]
        }"#;
        let resp: GraphQLResponse<Empty> = serde_json::from_str(json).unwrap();
        assert!(resp.data.is_none());
        let errors = resp.errors.unwrap();
        assert_eq!(errors[0].message, "Something went wrong");
    }
}
