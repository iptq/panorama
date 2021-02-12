#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    pub server: String,
    pub port: u16,

    pub username: String,
    pub password: String,
}
