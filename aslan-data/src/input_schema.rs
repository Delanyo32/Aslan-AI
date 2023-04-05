use std::collections::HashMap;
use apca::data::v2::bars::{Bars, Bar};

#[derive(Debug)]
pub struct AslanData {
    data_columns: HashMap<String,DataColumn>,
}
#[derive(Debug)]
pub struct DataColumn{
    data: Vec<DataEntry>,
}
#[derive(Debug)]
pub struct DataEntry{
    id: String,
    data: f64,
}

impl AslanData {
    pub fn new() -> Self {
        AslanData {
            data_columns: HashMap::new()
        }
    }

    pub fn parse_weeks(bars:Bars) -> Vec<Vec<Bar>>{
        let dst: Vec<Vec<Bar>> = bars.bars.chunks(7).map(|s| s.into()).collect();
        dst
    }

    pub fn partition_data(data:&Vec<f64>, size: usize)-> Vec<Vec<f64>>{
        let dst: Vec<Vec<f64>> = data.chunks(size).map(|s| s.into()).collect();
        dst
    }

    pub fn split_bar_components(bars:Bars) -> (Vec<f64>,Vec<f64>,Vec<f64>,Vec<f64>){
        let mut open_data = Vec::new();
        let mut high_data = Vec::new();
        let mut low_data = Vec::new();
        let mut close_data = Vec::new();
        for bar in bars.bars{
            open_data.push(bar.open.to_f64().unwrap());
            high_data.push(bar.high.to_f64().unwrap());
            low_data.push(bar.low.to_f64().unwrap());
            close_data.push(bar.close.to_f64().unwrap());
        }
        (open_data,high_data,low_data,close_data)
    }
    
    pub fn split_componets(data:Vec<Vec<Bar>>)->(Vec<Vec<f64>>,Vec<Vec<f64>>,Vec<Vec<f64>>,Vec<Vec<f64>>){
        let mut open_data = Vec::new();
        let mut high_data = Vec::new();
        let mut low_data = Vec::new();
        let mut close_data = Vec::new();
        for week in data {
            let mut week_data_open = Vec::new();
            let mut week_data_high = Vec::new();
            let mut week_data_low = Vec::new();
            let mut week_data_close = Vec::new();
    
            for bar in week {
                let open = bar.open.to_f64().unwrap();
                let high = bar.high.to_f64().unwrap();
                let low = bar.low.to_f64().unwrap();
                let close = bar.close.to_f64().unwrap();
    
                week_data_open.push(open);
                week_data_high.push(high);
                week_data_low.push(low);
                week_data_close.push(close);
            }
            open_data.push(week_data_open);
            high_data.push(week_data_high);
            low_data.push(week_data_low);
            close_data.push(week_data_close);
        }
        (open_data, high_data, low_data, close_data)
    }

    pub fn flat_vector(data:&Vec<Vec<f64>>)->Vec<f64>{
        let mut flat_data = Vec::new();
        for week in data {
            for day in week {
                let entry = day.to_owned();
                flat_data.push(entry);
            }
        }
        flat_data
    }

    pub fn add_column(mut self, key:String) -> Self {
        let data_column = DataColumn::new();
        self.data_columns.insert(key.to_string(), data_column);
        self
    }

    fn get_data(&self, key:String) -> &DataColumn {
        let column = &self.data_columns[&key];
        column
    }

    pub fn update_column(&mut self, data_entry:DataEntry, key:String) -> &Self {
        let data_column = match  self.data_columns.get_mut(&key) {
            Some(data_column) => data_column,
            None => panic!("No column with key {}", &key),
        };
        data_column.add_entry(data_entry);
        self
    }


    pub fn flatten_bar_data(self)->Vec<f64>{
        let mut data = Vec::new();
        let open_data = self.get_data("open".to_string());
        let high_data = self.get_data("high".to_string());
        let low_data = self.get_data("low".to_string());
        let close_data = self.get_data("close".to_string());
        for open in open_data.data.iter() {
            let high = match high_data.find_entry(open.id.to_string()){
                Some(high) => high,
                None => panic!("No high entry for id {}", &open.id),
            };
            let low = match low_data.find_entry(open.id.to_string()){
                Some(low) => low,
                None => panic!("No low entry for id {}", &open.id),
            };
            let close = match close_data.find_entry(open.id.to_string()) {
                Some(close) => close,
                None => panic!("No close entry for id {}", &open.id),
            };

            let average = (high.data + low.data) / 2.0;

            data.push(open.data);
            data.push(average);
            data.push(close.data);
        }
        data
    }


    pub fn parse_bars(mut self, bars:Bars) -> Self{
        for bar in bars.bars{
            let id = bar.time.to_string();
            let open = bar.open;
            let close = bar.close;
            let high = bar.high;
            let low = bar.low;

            let open_entry = DataEntry::new(id.to_string(), open.to_f64().unwrap());

            let close_entry = DataEntry::new(id.to_string(), close.to_f64().unwrap());

            let high_entry = DataEntry::new(id.to_string(), high.to_f64().unwrap());

            let low_entry = DataEntry::new(id.to_string(), low.to_f64().unwrap());

            self.update_column(open_entry, "open".to_string());
            self.update_column(close_entry, "close".to_string());
            self.update_column(high_entry, "high".to_string());
            self.update_column(low_entry, "low".to_string());

        }
        self
    }
}
impl DataColumn {
    fn new() -> Self {
        DataColumn {
            data: Vec::new(),
        }
    }

    fn add_entry(&mut self, data_entry:DataEntry) -> &Self {
        self.data.push(data_entry);
        self
    }

    fn find_entry(&self, id:String) -> Option<&DataEntry> {
        for entry in &self.data {
            if entry.id == id {
                return Some(entry);
            }
        }
        None
    }
}

impl DataEntry {
    fn new(id:String, data:f64) -> Self {
        DataEntry {
            id,
            data,
        }
    }
}

