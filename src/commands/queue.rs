use poise::CreateReply;

use crate::music::queue as music_queue;
use crate::utils::embed;
use crate::{Context, Error};

async fn queue_impl(ctx: Context<'_>, page: Option<usize>) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("서버에서만 사용할 수 있습니다")?;

    let (current, songs) = music_queue::get_queue_list(&ctx.data().queue_manager, guild_id).await;

    let page = page.unwrap_or(1);
    let embed = embed::queue_list(current.as_ref(), &songs, page);

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// 재생 목록을 표시합니다
#[poise::command(slash_command, guild_only)]
pub async fn queue(
    ctx: Context<'_>,
    #[description = "페이지 번호"] page: Option<usize>,
) -> Result<(), Error> {
    queue_impl(ctx, page).await
}

/// 재생 목록을 표시합니다 (/queue 단축)
#[poise::command(slash_command, guild_only)]
pub async fn q(
    ctx: Context<'_>,
    #[description = "페이지 번호"] page: Option<usize>,
) -> Result<(), Error> {
    queue_impl(ctx, page).await
}
