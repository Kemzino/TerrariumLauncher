use std::sync::{Arc, atomic::AtomicBool};

use discord_rich_presence::{
    DiscordIpc, DiscordIpcClient,
    activity::{Activity, Assets, Button, Timestamps},
};
use tokio::sync::RwLock;

use crate::State;

/// Terrarium's Discord application: its name is what Discord shows above the status.
const DISCORD_APP_ID: &str = "1551301026618015784";
/// Large image of the status: the pack icon, the animated planet (an art asset key of the app, or an https URL).
const DISCORD_LOGO: &str = "https://raw.githubusercontent.com/Kemzino/TerrariumLauncher/terrarium/apps/app-frontend/src/assets/terrarium/discord/planet.gif";
/// Our Discord server: the button under the status, and where the status lines link to.
const DISCORD_INVITE: &str = "https://discord.gg/S7NUwdGQHZ";

pub struct DiscordGuard {
    client: Arc<RwLock<DiscordIpcClient>>,
    connected: Arc<AtomicBool>,
    game: Arc<RwLock<Option<GameStatus>>>,
}

/// What the running game is doing, read from its log, and since when it runs.
#[derive(Clone)]
struct GameStatus {
    text: String,
    started: i64,
}

impl DiscordGuard {
    /// Initialize discord IPC client, and attempt to connect to it
    /// If it fails, it will still return a DiscordGuard, but the client will be unconnected
    pub fn init() -> crate::Result<DiscordGuard> {
        let dipc = DiscordIpcClient::new(DISCORD_APP_ID);

        Ok(DiscordGuard {
            client: Arc::new(RwLock::new(dipc)),
            connected: Arc::new(AtomicBool::new(false)),
            game: Arc::new(RwLock::new(None)),
        })
    }

    /// If the client failed connecting during init(), this will check for connection and attempt to reconnect
    /// This MUST be called first in any client method that requires a connection, because those can PANIC if the client is not connected
    /// (No connection is different than a failed connection, the latter will not panic and can be retried)
    pub async fn retry_if_not_ready(&self) -> bool {
        let mut client = self.client.write().await;
        if !self.connected.load(std::sync::atomic::Ordering::Relaxed) {
            if client.connect().is_ok() {
                self.connected
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                return true;
            }
            return false;
        }
        true
    }

    /// Set the activity to the given message
    /// First checks if discord is disabled, and if so, clear the activity instead
    pub async fn set_activity(
        &self,
        msg: &str,
        reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        // Check if discord is disabled, and if so, clear the activity instead
        let state = State::get().await?;
        let settings = crate::state::Settings::get(&state.pool).await?;
        if !settings.discord_rpc {
            Ok(self.clear_activity(true).await?)
        } else {
            Ok(self.force_set_activity(msg, reconnect_if_fail).await?)
        }
    }

    /// Sets the activity to the given message, regardless of if discord is disabled or offline
    /// Should not be used except for in the above method, or if it is already known that discord is enabled (specifically for state initialization) and we are connected to the internet
    pub async fn force_set_activity(
        &self,
        msg: &str,
        reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        // Attempt to connect if not connected. Do not continue if it fails, as the client.set_activity can panic if it never was connected
        if !self.retry_if_not_ready().await {
            return Ok(());
        }

        let started = self.game.read().await.as_ref().map(|game| game.started);
        let mut activity = Activity::new()
            .details("Terrarium Create")
            .details_url(DISCORD_INVITE)
            .state(msg)
            .state_url(DISCORD_INVITE)
            .assets(
                Assets::new()
                    .large_image(DISCORD_LOGO)
                    .large_text("Terrarium Create"),
            )
            .buttons(vec![Button::new("Discord сервер Terrarium", DISCORD_INVITE)]);
        if let Some(started) = started {
            activity = activity.timestamps(Timestamps::new().start(started));
        }

        // Attempt to set the activity
        // If the existing connection fails, attempt to reconnect and try again
        let mut client: tokio::sync::RwLockWriteGuard<'_, DiscordIpcClient> =
            self.client.write().await;
        let res = client.set_activity(activity.clone());

        if reconnect_if_fail {
            if let Err(_e) = res {
                client.reconnect()?;
                return Ok(client.set_activity(activity)?); // try again, but don't reconnect if it fails again
            }
        } else {
            res?;
        }

        Ok(())
    }

    /// A game was started: the status shows it (and the play time) until the game says more in its log.
    pub async fn game_started(&self, instance_name: &str) -> crate::Result<()> {
        let text = format!("Запускає {instance_name}");
        *self.game.write().await = Some(GameStatus {
            text: text.clone(),
            started: chrono::Utc::now().timestamp(),
        });
        self.set_activity(&text, true).await
    }

    /// The running game changed what it does (joined a server, opened a world).
    pub async fn game_status(&self, text: &str) -> crate::Result<()> {
        {
            let mut game = self.game.write().await;
            match game.as_mut() {
                Some(game) => game.text = text.to_string(),
                None => {
                    *game = Some(GameStatus {
                        text: text.to_string(),
                        started: chrono::Utc::now().timestamp(),
                    })
                }
            }
        }
        self.set_activity(text, true).await
    }

    /// Clear the activity entirely ('disabling' the RPC until the next set_activity)
    pub async fn clear_activity(
        &self,
        reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        // Attempt to connect if not connected. Do not continue if it fails, as the client.clear_activity can panic if it never was connected
        if !self.retry_if_not_ready().await {
            return Ok(());
        }

        // Attempt to clear the activity
        // If the existing connection fails, attempt to reconnect and try again
        let mut client = self.client.write().await;
        let res = client.clear_activity();

        if reconnect_if_fail {
            if res.is_err() {
                client.reconnect()?;
                return Ok(client.clear_activity()?); // try again, but don't reconnect if it fails again
            }
        } else {
            res?;
        }
        Ok(())
    }

    /// Clear the activity, but if there is a running profile, set the activity to that instead
    pub async fn clear_to_default(
        &self,
        reconnect_if_fail: bool,
    ) -> crate::Result<()> {
        let state = State::get().await?;

        let settings = crate::state::Settings::get(&state.pool).await?;
        if !settings.discord_rpc {
            println!("Discord is disabled, clearing activity");
            return self.clear_activity(true).await;
        }

        let running_instances = state.process_manager.get_all();
        if let Some(existing_child) = running_instances.first() {
            let text = self
                .game
                .read()
                .await
                .as_ref()
                .map(|game| game.text.clone())
                .unwrap_or_else(|| {
                    format!("Грає в {}", existing_child.instance_name)
                });
            self.set_activity(&text, reconnect_if_fail).await?;
        } else {
            *self.game.write().await = None;
            self.set_activity("У лаунчері", reconnect_if_fail).await?;
        }
        Ok(())
    }
}
