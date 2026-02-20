pub mod ammo;
pub mod boss;
pub mod craft;
pub mod hideout;
pub mod item;
pub mod map;
pub mod price;
pub mod quest;
pub mod questitem;
pub mod trader;

use crate::{Data, Error};

pub fn all() -> Vec<poise::Command<Data, Error>> {
    vec![
        item::item(),
        item::아이템(),
        price::price(),
        price::가격(),
        ammo::ammo(),
        ammo::탄약(),
        trader::trader(),
        trader::상인(),
        quest::quest(),
        quest::퀘스트(),
        questitem::questitem(),
        questitem::퀘스트아이템(),
        hideout::hideout(),
        hideout::은신처(),
        craft::craft(),
        craft::제작(),
        map::map(),
        map::맵(),
        boss::boss(),
        boss::보스(),
    ]
}
