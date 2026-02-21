use poise::CreateReply;
use serenity::builder::CreateEmbed;

use crate::{Context, Error};

async fn help_impl(ctx: Context<'_>) -> Result<(), Error> {
    let music_cmds = "\
`/play` (`/p`) — 음악 재생 또는 큐에 추가
`/skip` (`/s`) — 현재 곡 건너뛰기
`/stop` (`/st`) — 재생 중지 및 퇴장
`/queue` (`/q`) — 재생 목록 표시
`/pause` (`/pa`) — 일시정지
`/resume` (`/r`) — 재개
`/nowplaying` (`/np`) — 현재 재생 중인 곡
`/loop` (`/l`) — 반복 모드 (off/song/queue)
`/shuffle` (`/sh`) — 큐 셔플
`/remove` (`/rm`) — 큐에서 곡 제거
`/volume` (`/v`) — 볼륨 조절 (0-100)";

    let tarkov_cmds = "\
`/item` (`/아이템`) — 아이템 검색 (정보/가격 탭)
`/price` (`/가격`) — 벼룩시장 가격 조회
`/ammo` (`/탄약`) — 탄약 관통력/데미지 비교
`/trader` (`/상인`) — 트레이더 정보
`/quest` (`/퀘스트`) — 퀘스트 정보
`/questitem` (`/퀘스트아이템`) — 퀘스트 필요 아이템
`/hideout` (`/은신처`) — 은신처 정보
`/craft` (`/제작`) — 제작 레시피
`/map` (`/맵`) — 맵 정보
`/boss` (`/보스`) — 보스 정보";

    let embed = CreateEmbed::new()
        .title("EveryBot 도움말")
        .field("음악", music_cmds, false)
        .field("타르코프", tarkov_cmds, false)
        .color(0x5865F2);

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}

/// 봇 명령어 도움말
#[poise::command(slash_command, guild_only)]
pub async fn help(ctx: Context<'_>) -> Result<(), Error> {
    help_impl(ctx).await
}
