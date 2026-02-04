use serenity::all::User;

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub id: u64,
    pub username: String,
    pub nickname: Option<String>,
    pub avatar_url: Option<String>,
    pub is_bot: bool,
}

impl UserInfo {
    pub fn from_discord_user(user: &User) -> Self {
        Self {
            id: user.id.get(),
            username: user.name.clone(),
            nickname: None,
            avatar_url: user.avatar_url(),
            is_bot: user.bot,
        }
    }

    pub fn with_nickname(mut self, nickname: String) -> Self {
        self.nickname = Some(nickname);
        self
    }
}
