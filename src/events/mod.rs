pub mod voice_state;

use poise::serenity_prelude as serenity;

use crate::{Data, Error};

pub async fn handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    if let serenity::FullEvent::VoiceStateUpdate { old, new } = event {
        voice_state::handle(ctx, old, new, data).await?;
    }
    Ok(())
}
