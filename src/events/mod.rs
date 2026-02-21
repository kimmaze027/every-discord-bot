pub mod component;
pub mod voice_state;

use poise::serenity_prelude as serenity;

use crate::{Data, Error};

pub async fn handler(
    ctx: &serenity::Context,
    event: &serenity::FullEvent,
    _framework: poise::FrameworkContext<'_, Data, Error>,
    data: &Data,
) -> Result<(), Error> {
    match event {
        serenity::FullEvent::Message { new_message } => {
            crate::ai::channel::handle(ctx, new_message, data).await?;
        }
        serenity::FullEvent::VoiceStateUpdate { old, new } => {
            voice_state::handle(ctx, old, new, data).await?;
        }
        serenity::FullEvent::InteractionCreate {
            interaction: serenity::Interaction::Component(comp),
        } => {
            component::handle(ctx, comp, data).await?;
        }
        _ => {}
    }
    Ok(())
}
