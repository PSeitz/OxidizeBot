use crate::{
    api,
    auth::Scope,
    command,
    currency::Currency,
    module,
    prelude::*,
    utils::{Cooldown, Duration},
};
use anyhow::{bail, Error};
use std::collections::HashSet;

pub struct Handler {
    enabled: settings::Var<bool>,
    reward: settings::Var<i64>,
    cooldown: settings::Var<Cooldown>,
    currency: injector::Var<Option<Currency>>,
    twitch: api::Twitch,
}

#[async_trait]
impl command::Handler for Handler {
    fn scope(&self) -> Option<Scope> {
        Some(Scope::SwearJar)
    }

    async fn handle(&self, ctx: &mut command::Context) -> Result<(), Error> {
        if !self.enabled.load().await {
            return Ok(());
        }

        let currency = match self.currency.load().await {
            Some(currency) => currency,
            None => {
                respond!(ctx, "No currency configured for stream, sorry :(");
                return Ok(());
            }
        };

        if !self.cooldown.write().await.is_open() {
            respond!(
                ctx,
                "A !swearjar command was recently issued, please wait a bit longer!"
            );
            return Ok(());
        }

        let user = &ctx.user;
        let reward = self.reward.load().await;

        let chatters = self.twitch.chatters(user.channel()).await?;

        let mut u = HashSet::new();
        u.extend(chatters.viewers);
        u.extend(chatters.moderators);

        if u.is_empty() {
            bail!("no chatters to reward");
        }

        let total_reward = reward * u.len() as i64;

        currency
            .balance_add(user.channel(), &user.streamer().name, -total_reward)
            .await?;

        currency
            .balances_increment(user.channel(), u, reward, 0)
            .await?;

        user.sender().privmsg(format!(
            "/me has taken {} {currency} from {streamer} and given it to the viewers for listening to their bad mouth!",
            total_reward, currency = currency.name, streamer = user.streamer().display_name,
        )).await;

        Ok(())
    }
}

pub struct Module;

#[async_trait]
impl super::Module for Module {
    fn ty(&self) -> &'static str {
        "swearjar"
    }

    /// Set up command handlers for this module.
    async fn hook(
        &self,
        module::HookContext {
            handlers,
            twitch,
            injector,
            futures,
            settings,
            ..
        }: module::HookContext<'_>,
    ) -> Result<(), Error> {
        let enabled = settings.var("swearjar/enabled", false).await?;
        let reward = settings.var("swearjar/reward", 10).await?;

        let (mut cooldown_stream, cooldown) = settings
            .stream("swearjar/cooldown")
            .or_with(Duration::seconds(60 * 10))
            .await?;

        let cooldown = settings::Var::new(Cooldown::from_duration(cooldown));

        let currency = injector.var().await?;

        handlers.insert(
            "swearjar",
            Handler {
                enabled,
                reward,
                cooldown: cooldown.clone(),
                currency,
                twitch: twitch.clone(),
            },
        );

        let future = async move {
            loop {
                futures::select! {
                    update = cooldown_stream.select_next_some() => {
                        cooldown.write().await.cooldown = update;
                    }
                }
            }
        };

        futures.push(future.boxed());
        Ok(())
    }
}
