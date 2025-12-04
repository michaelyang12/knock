use dotenvy::dotenv;

pub fn init() -> String {
    dotenv().ok();

    std::env::var("API_KEY").expect("API_KEY missing")
}
