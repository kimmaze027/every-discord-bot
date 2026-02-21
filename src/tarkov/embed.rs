use serenity::builder::{CreateEmbed, CreateEmbedFooter};

use super::models::{Ammo, Boss, Craft, GameMap, HideoutStation, Item, Task, Trader};

/// Consistent color for all Tarkov embeds (dark gold).
const TARKOV_COLOR: u32 = 0xC8AA6E;

/// Items per page for ammo table and quest item list.
const ITEMS_PER_PAGE: usize = 10;

/// Format a number with comma separators for Korean locale display.
fn format_number(n: i64) -> String {
    if n == 0 {
        return "0".to_string();
    }

    let negative = n < 0;
    let mut num = n.unsigned_abs();
    let mut parts = Vec::new();

    while num > 0 {
        parts.push(format!("{:03}", num % 1000));
        num /= 1000;
    }

    parts.reverse();

    // Remove leading zeros from the first group.
    if let Some(first) = parts.first_mut() {
        *first = first.trim_start_matches('0').to_string();
        if first.is_empty() {
            *first = "0".to_string();
        }
    }

    let result = parts.join(",");
    if negative {
        format!("-{result}")
    } else {
        result
    }
}

/// Format duration in seconds to a human-readable Korean string.
fn format_duration(seconds: i32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;

    if hours > 0 && minutes > 0 {
        format!("{hours}시간 {minutes}분")
    } else if hours > 0 {
        format!("{hours}시간")
    } else if minutes > 0 {
        format!("{minutes}분")
    } else {
        format!("{seconds}초")
    }
}

/// Item detail embed showing basic info, stats, and category.
pub fn item_detail(item: &Item) -> CreateEmbed {
    let mut description = String::new();

    if let Some(ref desc) = item.description {
        description.push_str(desc);
        description.push('\n');
    }

    description.push_str(&format!("\n**단축명**: {}", item.short_name));

    let categories: Vec<&str> = item.categories.iter().map(|c| c.name.as_str()).collect();
    if !categories.is_empty() {
        description.push_str(&format!("\n**카테고리**: {}", categories.join(", ")));
    }

    description.push_str(&format!(
        "\n**기본가**: {} RUB",
        format_number(item.base_price)
    ));
    description.push_str(&format!("\n**무게**: {:.2} kg", item.weight));
    description.push_str(&format!("\n**크기**: {}x{}", item.width, item.height));

    let mut embed = CreateEmbed::new()
        .title(&item.name)
        .description(description)
        .color(TARKOV_COLOR);

    if let Some(ref img) = item.grid_image_link {
        embed = embed.thumbnail(img);
    }

    embed
}

/// Item price embed showing 24h flea market prices and vendor sell prices.
pub fn item_price(item: &Item) -> CreateEmbed {
    let mut description = String::new();

    description.push_str("**벼룩시장 (24시간)**\n");

    match item.avg24h_price {
        Some(avg) if avg > 0 => {
            description.push_str(&format!("  평균: {} RUB\n", format_number(avg)));
            if let Some(low) = item.low24h_price {
                description.push_str(&format!("  최저: {} RUB\n", format_number(low)));
            }
            if let Some(high) = item.high24h_price {
                description.push_str(&format!("  최고: {} RUB\n", format_number(high)));
            }
        }
        _ => {
            description.push_str("  거래 불가 또는 데이터 없음\n");
        }
    }

    if !item.sell_for.is_empty() {
        description.push_str("\n**상인 매입가**\n");
        for vendor_price in &item.sell_for {
            description.push_str(&format!(
                "  {}: {} {}\n",
                vendor_price.vendor.name,
                format_number(vendor_price.price),
                vendor_price.currency,
            ));
        }
    }

    let mut embed = CreateEmbed::new()
        .title(format!("{} - 가격 정보", item.name))
        .description(description)
        .color(TARKOV_COLOR);

    if let Some(ref img) = item.icon_link {
        embed = embed.thumbnail(img);
    }

    embed
}

/// Ammo comparison table embed with pagination.
///
/// `sort` is displayed in the footer (e.g., "관통력순").
/// `page` is 0-indexed.
pub fn ammo_table(ammo_list: &[Ammo], sort: &str, page: usize) -> CreateEmbed {
    let total_pages = if ammo_list.is_empty() {
        1
    } else {
        ammo_list.len().div_ceil(ITEMS_PER_PAGE)
    };
    let page = page.min(total_pages.saturating_sub(1));

    let start = page * ITEMS_PER_PAGE;
    let end = (start + ITEMS_PER_PAGE).min(ammo_list.len());

    let mut description = String::new();
    description.push_str("```\n");
    description.push_str(&format!(
        "{:<20} {:>4} {:>4} {:>4}\n",
        "이름", "관통", "데미지", "방피"
    ));
    description.push_str(&"-".repeat(36));
    description.push('\n');

    for ammo in ammo_list.get(start..end).unwrap_or(&[]) {
        let name = if ammo.item.short_name.len() > 18 {
            format!("{}...", &ammo.item.short_name[..15])
        } else {
            ammo.item.short_name.clone()
        };
        description.push_str(&format!(
            "{:<20} {:>4} {:>6} {:>4}\n",
            name, ammo.penetration_power, ammo.damage, ammo.armor_damage,
        ));
    }
    description.push_str("```");

    CreateEmbed::new()
        .title(format!("탄약 비교 ({}/{})", page + 1, total_pages))
        .description(description)
        .color(TARKOV_COLOR)
        .footer(CreateEmbedFooter::new(format!(
            "정렬: {sort} | 총 {}발",
            ammo_list.len()
        )))
}

/// Trader info embed with level requirements table.
pub fn trader_info(trader: &Trader) -> CreateEmbed {
    let mut description = String::new();

    if let Some(ref desc) = trader.description {
        description.push_str(desc);
        description.push_str("\n\n");
    }

    description.push_str(&format!("**거래 화폐**: {}\n", trader.currency.name));

    if let Some(ref reset) = trader.reset_time {
        description.push_str(&format!("**리셋 시간**: {reset}\n"));
    }

    if !trader.levels.is_empty() {
        description.push_str("\n**레벨 요구사항**\n```\n");
        description.push_str(&format!(
            "{:<4} {:>6} {:>6} {:>12}\n",
            "Lv", "플레이어", "평판", "거래액"
        ));
        description.push_str(&"-".repeat(32));
        description.push('\n');

        for level in &trader.levels {
            description.push_str(&format!(
                "{:<4} {:>6} {:>6.2} {:>12}\n",
                level.level,
                level.required_player_level,
                level.required_reputation,
                format_number(level.required_commerce),
            ));
        }
        description.push_str("```");
    }

    let mut embed = CreateEmbed::new()
        .title(&trader.name)
        .description(description)
        .color(TARKOV_COLOR);

    if let Some(ref img) = trader.image_link {
        embed = embed.thumbnail(img);
    }

    embed
}

/// Quest info embed with objectives, rewards, and quest chain visualization.
///
/// `prev_quests` and `next_quests` are name strings for chain display.
pub fn quest_info(task: &Task, prev_quests: &[&str], next_quests: &[&str]) -> CreateEmbed {
    let mut description = String::new();

    description.push_str(&format!("**상인**: {}\n", task.trader.name));

    if let Some(ref map) = task.map {
        description.push_str(&format!("**맵**: {}\n", map.name));
    }

    if let Some(level) = task.min_player_level {
        description.push_str(&format!("**최소 레벨**: {level}\n"));
    }

    // Objectives
    if !task.objectives.is_empty() {
        description.push_str("\n**목표**\n");
        for obj in &task.objectives {
            let prefix = if obj.optional { "(선택) " } else { "" };
            description.push_str(&format!("- {prefix}{}\n", obj.description));
        }
    }

    // Rewards
    if let Some(ref rewards) = task.finish_rewards {
        if !rewards.items.is_empty() {
            description.push_str("\n**완료 보상**\n");
            for reward in &rewards.items {
                description.push_str(&format!("- {} x{}\n", reward.item.name, reward.count));
            }
        }
    }

    // Quest chain: previous
    if !prev_quests.is_empty() {
        description.push_str("\n**이전 퀘스트**\n");
        for name in prev_quests {
            description.push_str(&format!("- {name}\n"));
        }
    }

    // Quest chain: next
    if !next_quests.is_empty() {
        description.push_str("\n**다음 퀘스트**\n");
        for name in next_quests {
            description.push_str(&format!("- {name}\n"));
        }
    }

    CreateEmbed::new()
        .title(&task.name)
        .description(description)
        .color(TARKOV_COLOR)
}

/// Quest item list embed showing quests that require a specific item.
///
/// `quests` is a list of (quest_name, trader_name).
/// `page` is 0-indexed.
pub fn questitem_list(quests: &[(String, String)], item_name: &str, page: usize) -> CreateEmbed {
    let total_pages = if quests.is_empty() {
        1
    } else {
        quests.len().div_ceil(ITEMS_PER_PAGE)
    };
    let page = page.min(total_pages.saturating_sub(1));

    let start = page * ITEMS_PER_PAGE;
    let end = (start + ITEMS_PER_PAGE).min(quests.len());

    let mut description = String::new();

    if quests.is_empty() {
        description.push_str("해당 아이템이 필요한 퀘스트가 없습니다.");
    } else {
        for (quest_name, trader_name) in quests.get(start..end).unwrap_or(&[]) {
            description.push_str(&format!("- **{quest_name}** ({trader_name})\n"));
        }
    }

    CreateEmbed::new()
        .title(format!(
            "\"{item_name}\" 필요 퀘스트 ({}/{})",
            page + 1,
            total_pages,
        ))
        .description(description)
        .color(TARKOV_COLOR)
        .footer(CreateEmbedFooter::new(format!(
            "총 {}개 퀘스트",
            quests.len()
        )))
}

/// Map info embed showing raid duration, players, extracts, and bosses.
pub fn map_info(map: &GameMap) -> CreateEmbed {
    let mut description = String::new();

    if let Some(ref desc) = map.description {
        description.push_str(desc);
        description.push_str("\n\n");
    }

    if let Some(duration) = map.raid_duration {
        description.push_str(&format!("**레이드 시간**: {duration}분\n"));
    }

    if let Some(ref players) = map.players {
        description.push_str(&format!("**플레이어 수**: {players}\n"));
    }

    // Extracts
    if !map.extracts.is_empty() {
        description.push_str("\n**탈출구**\n");
        for extract in &map.extracts {
            let faction_label = match extract.faction.as_deref() {
                Some("pmc") => " (PMC)",
                Some("scav") => " (Scav)",
                Some("shared") => " (공용)",
                Some(other) => {
                    // Use a temporary binding to extend lifetime
                    let label = format!(" ({other})");
                    description.push_str(&format!("- {}{}\n", extract.name, label));
                    continue;
                }
                None => "",
            };
            description.push_str(&format!("- {}{faction_label}\n", extract.name));
        }
    }

    // Bosses
    if !map.bosses.is_empty() {
        description.push_str("\n**보스**\n");
        for boss in &map.bosses {
            let chance = (boss.spawn_chance * 100.0) as i32;
            description.push_str(&format!("- {} (스폰 확률: {chance}%)\n", boss.name));
        }
    }

    CreateEmbed::new()
        .title(&map.name)
        .description(description)
        .color(TARKOV_COLOR)
}

/// Boss info embed with tabs support.
///
/// `tab` determines which view to show: "info", "equip", "drops", "spawns".
/// `spawn_maps` is a list of (map_name, spawn_chance) from maps query cross-reference.
pub fn boss_info(boss: &Boss, tab: &str, spawn_maps: &[(String, f64)]) -> CreateEmbed {
    let mut embed = CreateEmbed::new().title(&boss.name).color(TARKOV_COLOR);

    if let Some(ref img) = boss.image_poster_link {
        embed = embed.thumbnail(img);
    }

    let description = match tab {
        "info" => {
            let mut desc = String::new();
            // Total health
            if let Some(ref health_parts) = boss.health {
                let total: i32 = health_parts.iter().map(|h| h.max).sum();
                desc.push_str(&format!("**총 체력**: {total} HP\n"));

                if health_parts.len() > 1 {
                    desc.push_str("**부위별 체력**\n");
                    for (i, h) in health_parts.iter().enumerate() {
                        desc.push_str(&format!("  부위 {}: {} HP\n", i + 1, h.max));
                    }
                }
            }
            desc
        }
        "equip" => {
            let mut desc = String::new();
            if boss.equipment.is_empty() {
                desc.push_str("장비 정보가 없습니다.");
            } else {
                desc.push_str("**장비 목록**\n");
                for equip in &boss.equipment {
                    desc.push_str(&format!("- {}\n", equip.item.name));
                }
            }
            desc
        }
        "drops" => {
            let mut desc = String::new();
            if boss.items.is_empty() {
                desc.push_str("드롭 아이템 정보가 없습니다.");
            } else {
                desc.push_str("**드롭 아이템**\n");
                for loot in &boss.items {
                    desc.push_str(&format!("- {}\n", loot.item.name));
                }
            }
            desc
        }
        "spawns" => {
            let mut desc = String::new();
            if spawn_maps.is_empty() {
                desc.push_str("스폰 위치 정보가 없습니다.");
            } else {
                desc.push_str("**스폰 위치**\n");
                for (map_name, chance) in spawn_maps {
                    let pct = (*chance * 100.0) as i32;
                    desc.push_str(&format!("- {map_name} (스폰 확률: {pct}%)\n"));
                }
            }
            desc
        }
        _ => "알 수 없는 탭입니다.".to_string(),
    };

    embed.description(description)
}

/// Hideout station info embed with a specific level's detail.
///
/// `level_idx` is 0-indexed into `station.levels`.
pub fn hideout_info(station: &HideoutStation, level_idx: usize) -> CreateEmbed {
    let total_levels = station.levels.len();
    let level_idx = level_idx.min(total_levels.saturating_sub(1));

    let mut description = String::new();

    if let Some(level) = station.levels.get(level_idx) {
        description.push_str(&format!("**레벨 {}**\n\n", level.level));
        description.push_str(&format!(
            "**건설 시간**: {}\n",
            format_duration(level.construction_time)
        ));

        // Item requirements
        if !level.item_requirements.is_empty() {
            description.push_str("\n**필요 아이템**\n");
            for req in &level.item_requirements {
                description.push_str(&format!("- {} x{}\n", req.item.name, req.count));
            }
        }

        // Station level requirements
        if !level.station_level_requirements.is_empty() {
            description.push_str("\n**필요 시설**\n");
            for req in &level.station_level_requirements {
                description.push_str(&format!("- {} Lv.{}\n", req.station.name, req.level));
            }
        }
    } else {
        description.push_str("레벨 정보가 없습니다.");
    }

    CreateEmbed::new()
        .title(format!(
            "{} ({}/{})",
            station.name,
            level_idx + 1,
            total_levels.max(1),
        ))
        .description(description)
        .color(TARKOV_COLOR)
}

/// Craft recipe embed showing materials, costs, duration, and station.
pub fn craft_info(craft: &Craft) -> CreateEmbed {
    let mut description = String::new();

    description.push_str(&format!(
        "**제작소**: {} Lv.{}\n",
        craft.station.name, craft.level
    ));
    description.push_str(&format!(
        "**제작 시간**: {}\n",
        format_duration(craft.duration)
    ));

    // Produced items
    if !craft.reward_items.is_empty() {
        description.push_str("\n**생산품**\n");
        for item in &craft.reward_items {
            let price_str = item
                .item
                .avg24h_price
                .map(|p| format!(" (~{} RUB)", format_number(p)))
                .unwrap_or_default();
            description.push_str(&format!(
                "- {} x{}{price_str}\n",
                item.item.name, item.count,
            ));
        }
    }

    // Required materials
    if !craft.required_items.is_empty() {
        description.push_str("\n**필요 재료**\n");
        let mut total_cost: i64 = 0;
        for item in &craft.required_items {
            let price_str = match item.item.avg24h_price {
                Some(p) if p > 0 => {
                    total_cost += p * i64::from(item.count);
                    format!(" (~{} RUB)", format_number(p * i64::from(item.count)))
                }
                _ => String::new(),
            };
            description.push_str(&format!(
                "- {} x{}{price_str}\n",
                item.item.name, item.count,
            ));
        }

        if total_cost > 0 {
            description.push_str(&format!(
                "\n**총 재료 비용**: ~{} RUB",
                format_number(total_cost)
            ));
        }
    }

    let reward_name = craft
        .reward_items
        .first()
        .map(|r| r.item.name.as_str())
        .unwrap_or("제작");

    CreateEmbed::new()
        .title(format!("{reward_name} 제작법"))
        .description(description)
        .color(TARKOV_COLOR)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tarkov::models::*;

    fn sample_item() -> Item {
        Item {
            id: "abc123".into(),
            name: "LEDX".into(),
            short_name: "LEDX".into(),
            base_price: 100_000,
            weight: 0.5,
            width: 1,
            height: 1,
            description: Some("의료 장비".into()),
            grid_image_link: Some("https://example.com/img.png".into()),
            icon_link: Some("https://example.com/icon.png".into()),
            avg24h_price: Some(1_500_000),
            low24h_price: Some(1_200_000),
            high24h_price: Some(1_800_000),
            sell_for: vec![VendorPrice {
                vendor: Vendor {
                    name: "치료사".into(),
                },
                price: 300_000,
                currency: "RUB".into(),
            }],
            categories: vec![Category {
                name: "의료".into(),
            }],
        }
    }

    fn sample_ammo() -> Ammo {
        Ammo {
            item: AmmoItem {
                id: "ammo1".into(),
                name: "7.62x39mm BP".into(),
                short_name: "BP".into(),
                icon_link: None,
                grid_image_link: None,
            },
            caliber: "7.62x39mm".into(),
            damage: 58,
            armor_damage: 47,
            penetration_power: 47,
            penetration_chance: 0.93,
            ricochet_chance: 0.05,
            fragmentation_chance: 0.12,
            projectile_count: None,
        }
    }

    fn sample_trader() -> Trader {
        Trader {
            id: "t1".into(),
            name: "프라포르".into(),
            description: Some("무기 거래상".into()),
            image_link: Some("https://example.com/prapor.png".into()),
            reset_time: Some("2h".into()),
            levels: vec![
                TraderLevel {
                    level: 1,
                    required_player_level: 1,
                    required_reputation: 0.0,
                    required_commerce: 0,
                },
                TraderLevel {
                    level: 2,
                    required_player_level: 15,
                    required_reputation: 0.2,
                    required_commerce: 1_000_000,
                },
            ],
            currency: Currency { name: "RUB".into() },
        }
    }

    fn sample_task() -> Task {
        Task {
            id: "task1".into(),
            name: "보급품 조달".into(),
            trader: TaskTrader {
                name: "프라포르".into(),
            },
            map: Some(TaskMap {
                name: "세관".into(),
            }),
            min_player_level: Some(5),
            objectives: vec![
                TaskObjective {
                    description: "아이템 3개 찾기".into(),
                    optional: false,
                },
                TaskObjective {
                    description: "보너스 목표".into(),
                    optional: true,
                },
            ],
            finish_rewards: Some(TaskRewards {
                items: vec![TaskRewardItem {
                    item: RewardItemInfo {
                        name: "루블".into(),
                    },
                    count: 50000,
                }],
            }),
            task_requirements: vec![TaskRequirement {
                task: TaskRequirementRef {
                    id: "task0".into(),
                    name: "첫 번째 퀘스트".into(),
                },
            }],
        }
    }

    fn sample_map() -> GameMap {
        GameMap {
            id: "map1".into(),
            name: "세관".into(),
            description: Some("위험한 지역".into()),
            raid_duration: Some(45),
            players: Some("8-12".into()),
            bosses: vec![MapBoss {
                name: "레쉴라".into(),
                spawn_chance: 0.38,
            }],
            extracts: vec![
                MapExtract {
                    name: "출구 A".into(),
                    faction: Some("pmc".into()),
                },
                MapExtract {
                    name: "출구 B".into(),
                    faction: None,
                },
            ],
        }
    }

    fn sample_boss() -> Boss {
        Boss {
            name: "킬라".into(),
            health: Some(vec![BossHealth { max: 890 }, BossHealth { max: 70 }]),
            image_poster_link: Some("https://example.com/killa.png".into()),
            equipment: vec![
                BossEquipment {
                    item: EquipmentItem {
                        name: "RPK-16".into(),
                    },
                },
                BossEquipment {
                    item: EquipmentItem {
                        name: "Maska-1Sch".into(),
                    },
                },
            ],
            items: vec![BossLootItem {
                item: LootItemInfo {
                    name: "킬라의 헬멧".into(),
                },
            }],
        }
    }

    fn sample_hideout() -> HideoutStation {
        HideoutStation {
            id: "s1".into(),
            name: "작업대".into(),
            levels: vec![HideoutLevel {
                level: 1,
                construction_time: 3600,
                item_requirements: vec![HideoutItemReq {
                    item: RewardItemInfo {
                        name: "볼트".into(),
                    },
                    count: 3,
                }],
                station_level_requirements: vec![StationLevelReq {
                    station: StationRef {
                        name: "발전기".into(),
                    },
                    level: 1,
                }],
            }],
        }
    }

    fn sample_craft() -> Craft {
        Craft {
            id: "c1".into(),
            duration: 7200,
            station: CraftStation {
                name: "작업대".into(),
            },
            level: 2,
            reward_items: vec![CraftItem {
                item: CraftItemInfo {
                    name: "총기 부품".into(),
                    avg24h_price: Some(50_000),
                },
                count: 1,
            }],
            required_items: vec![
                CraftItem {
                    item: CraftItemInfo {
                        name: "볼트".into(),
                        avg24h_price: Some(10_000),
                    },
                    count: 3,
                },
                CraftItem {
                    item: CraftItemInfo {
                        name: "너트".into(),
                        avg24h_price: None,
                    },
                    count: 5,
                },
            ],
        }
    }

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(999), "999");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1_500_000), "1,500,000");
        assert_eq!(format_number(-1234), "-1,234");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30), "30초");
        assert_eq!(format_duration(300), "5분");
        assert_eq!(format_duration(3600), "1시간");
        assert_eq!(format_duration(5400), "1시간 30분");
    }

    #[test]
    fn test_item_detail_creates() {
        let item = sample_item();
        let _embed = item_detail(&item);
    }

    #[test]
    fn test_item_detail_no_description() {
        let mut item = sample_item();
        item.description = None;
        item.grid_image_link = None;
        let _embed = item_detail(&item);
    }

    #[test]
    fn test_item_price_creates() {
        let item = sample_item();
        let _embed = item_price(&item);
    }

    #[test]
    fn test_item_price_no_flea_data() {
        let mut item = sample_item();
        item.avg24h_price = None;
        item.low24h_price = None;
        item.high24h_price = None;
        item.sell_for = vec![];
        let _embed = item_price(&item);
    }

    #[test]
    fn test_ammo_table_creates() {
        let ammo_list = vec![sample_ammo()];
        let _embed = ammo_table(&ammo_list, "관통력순", 0);
    }

    #[test]
    fn test_ammo_table_empty() {
        let _embed = ammo_table(&[], "관통력순", 0);
    }

    #[test]
    fn test_ammo_table_pagination() {
        let ammo_list: Vec<Ammo> = (0..25).map(|_| sample_ammo()).collect();
        let _embed = ammo_table(&ammo_list, "데미지순", 1);
    }

    #[test]
    fn test_trader_info_creates() {
        let trader = sample_trader();
        let _embed = trader_info(&trader);
    }

    #[test]
    fn test_quest_info_creates() {
        let task = sample_task();
        let _embed = quest_info(&task, &["첫 번째 퀘스트"], &["세 번째 퀘스트"]);
    }

    #[test]
    fn test_quest_info_no_chain() {
        let task = sample_task();
        let _embed = quest_info(&task, &[], &[]);
    }

    #[test]
    fn test_questitem_list_creates() {
        let quests = vec![
            ("보급품 조달".into(), "프라포르".into()),
            ("두 번째 퀘스트".into(), "치료사".into()),
        ];
        let _embed = questitem_list(&quests, "LEDX", 0);
    }

    #[test]
    fn test_questitem_list_empty() {
        let _embed = questitem_list(&[], "없는아이템", 0);
    }

    #[test]
    fn test_map_info_creates() {
        let map = sample_map();
        let _embed = map_info(&map);
    }

    #[test]
    fn test_boss_info_all_tabs() {
        let boss = sample_boss();
        let spawns = vec![("인터체인지".into(), 0.35)];

        let _embed_info = boss_info(&boss, "info", &spawns);
        let _embed_equip = boss_info(&boss, "equip", &spawns);
        let _embed_drops = boss_info(&boss, "drops", &spawns);
        let _embed_spawns = boss_info(&boss, "spawns", &spawns);
    }

    #[test]
    fn test_boss_info_unknown_tab() {
        let boss = sample_boss();
        let _embed = boss_info(&boss, "unknown", &[]);
    }

    #[test]
    fn test_boss_info_empty_data() {
        let boss = Boss {
            name: "테스트 보스".into(),
            health: None,
            image_poster_link: None,
            equipment: vec![],
            items: vec![],
        };
        let _embed = boss_info(&boss, "info", &[]);
        let _embed = boss_info(&boss, "equip", &[]);
        let _embed = boss_info(&boss, "drops", &[]);
        let _embed = boss_info(&boss, "spawns", &[]);
    }

    #[test]
    fn test_hideout_info_creates() {
        let station = sample_hideout();
        let _embed = hideout_info(&station, 0);
    }

    #[test]
    fn test_hideout_info_out_of_bounds() {
        let station = sample_hideout();
        // Should clamp to last valid index, not panic.
        let _embed = hideout_info(&station, 100);
    }

    #[test]
    fn test_craft_info_creates() {
        let craft = sample_craft();
        let _embed = craft_info(&craft);
    }

    #[test]
    fn test_craft_info_no_prices() {
        let mut craft = sample_craft();
        for item in &mut craft.required_items {
            item.item.avg24h_price = None;
        }
        for item in &mut craft.reward_items {
            item.item.avg24h_price = None;
        }
        let _embed = craft_info(&craft);
    }
}
