use std::{path};
use apca::{Client,ApiInfo};
use serde::{Serialize, Deserialize};
use chrono::{Utc, TimeZone,DateTime};
use apca::data::v2::bars::{TimeFrame,Bars,Get,BarsReqInit};

#[derive(Serialize, Deserialize)]
struct AlpacaConfig{
    api_key: String,
    api_secret: String,
    api_base_url: String,
}

pub struct DataConfig{
    symbol: String,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
}

impl DataConfig{
    pub fn new(symbol: String, start_date: (i32,u32,u32), end_date: (i32,u32,u32)) -> DataConfig{
        DataConfig{
            symbol,
            start_date : Utc.ymd(start_date.0,start_date.1,start_date.2).and_hms(0,0,0),
            end_date: Utc.ymd(end_date.0,end_date.1,end_date.2).and_hms(0,0,0),
        }
    }
}

pub struct AlpacaData {
    alpaca_client: Client,
}

impl AlpacaData {
    pub fn new() -> Self {
        let config: AlpacaConfig = match confy::load_path(path::Path::new("config.toml")) {
            Ok(config) => config,
            Err(e) => panic!("Error loading config Please create/edit the config.toml file in the root of the project with your alpaca details: {}", e),
        };

        println!("{:?}", config.api_base_url);
        let api_info = match ApiInfo::from_parts(config.api_base_url, config.api_key, config.api_secret) {
            Ok(api_info) => api_info,
            Err(e) => panic!("Error creating api info: {}", e),
        };
        let client = apca::Client::new(api_info);
        AlpacaData {
            alpaca_client: client,
        }
    }

    pub async fn fetch_data(&self,data_config:DataConfig)->Bars{
        let request = BarsReqInit::default().init(data_config.symbol, data_config.start_date, data_config.end_date, TimeFrame::OneMinute);

        let bars = match self.alpaca_client.issue::<Get>(&request).await {
            Ok(bars) => bars,
            Err(e) => panic!("Error fetching data: {}", e),
        };
        bars
    }
}

impl Default for AlpacaConfig {
    fn default() -> Self {
        AlpacaConfig {
            api_key: String::from(""),
            api_secret: String::from(""),
            api_base_url: String::from(""),
        }
    }
}
    