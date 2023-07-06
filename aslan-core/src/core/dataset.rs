
use std::collections::HashMap;
use burn_ndarray::{NdArrayBackend, NdArrayDevice};
use crate::{db::mongodb::MongoClient, helpers::dataparser::TestData};
use burn::{tensor::{backend::Backend, Tensor, Int, Data, ElementConversion}, data::{dataset::{DatasetIterator, Dataset}, dataloader::batcher::Batcher}};
use log::info;



// Dataset
pub struct AslanDataset {
    pub data: Vec<TestData>,
}

impl AslanDataset {
    pub async fn train() -> Self {
        let mongo_client = MongoClient::new().await;
        let data = mongo_client.get_test_data("test-dataset".to_owned(), "test-data".to_owned()).await;
        Self {
            data,
        }
    }
    pub async fn validate() -> Self {
        let mongo_client = MongoClient::new().await;
        let data = mongo_client.get_test_data("validation-dataset".to_owned(), "validation-data".to_owned()).await;
        Self {
            data,
        }
    }
}

impl Dataset<TestData> for AslanDataset {

    fn len(&self) -> usize {
        self.data.len()
    }

    fn get(&self, index: usize) -> Option<TestData> {
        self.data.get(index).cloned()
    }

    fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    fn iter(&self) -> DatasetIterator<'_, TestData> {
        DatasetIterator::new(self)
    }
}

pub struct AslanDataBatcher<B: Backend> {
    device: B::Device,
    tensor_map: HashMap<String, Tensor<B, 1>>,
    label_map: HashMap<String, i64>,
}

impl<B: Backend> AslanDataBatcher<B> {
    pub fn new(device: B::Device) -> Self {
        Self { 
            device : device,
            tensor_map: HashMap::new(), 
            label_map: HashMap::new(),
        }
    }

    pub async fn initialize_map(&self)-> Self{
        let (tensor_map, label_map) = generate_tensor_map::<B>().await;
        Self { 
            device: self.device.clone(),
            tensor_map,
            label_map
        }
    }
}

#[derive(Clone, Debug)]
pub struct DataBatch<B: Backend> {
    pub inputs: Tensor<B, 2>,
    pub outputs: Tensor<B, 1, Int>,
}

impl<B: Backend> DataBatch<B> {
    pub fn new(inputs: Tensor<B, 2>, outputs: Tensor<B, 1, Int>) -> Self {
        Self { 
            inputs,
            outputs,
        }
    }
}
// dataset 
// - gets list of unique tokens
// - convert the tokens to tensors 0.1 -> [0,0,1,0,0,0,0,0,0,0] if 0.1 is the 3rd token in the list

impl<B: Backend> Batcher<TestData, DataBatch<B>> for AslanDataBatcher<B> {
    fn batch(&self, items: Vec<TestData>) -> DataBatch<B> {

        let mut input_data = Vec::new();
        let mut output_data = Vec::new();
        let tensor_size = self.tensor_map.len();
        let batch_size = items.len();
        for entry in items{
            let input = entry.input_data.to_string();
            let output = entry.output_data.to_string();
            let input_tensor = self.tensor_map.get(&input).unwrap();
            let output_int = self.label_map.get(&output).unwrap();
            let output_tensor = Tensor::<B, 1, Int>::from_data(Data::from([output_int.elem()]));
            input_data.push(input_tensor.clone());
            output_data.push(output_tensor.clone());
        }
    
        // convert vec to tensor
        let input_tensor = Tensor::cat(input_data, 0).to_device(&self.device);
        let input_tensor = input_tensor.reshape([batch_size,tensor_size]);


        let output_tensor = Tensor::cat(output_data, 0).to_device(&self.device);
    
        // create a data batch
        let data_batch = DataBatch::<B>::new(input_tensor, output_tensor);
        info!("data batch: {:?}", data_batch);
    
        data_batch
    }
}




pub async fn generate_tensor_map<B: Backend>()->(HashMap<String, Tensor::<B, 1>>, HashMap<String, i64>){
    let mongo_client = MongoClient::new().await;
    let db_embeddings = mongo_client.get_embeddings().await;
    
    // create hashmap for the labels and tensors
    let mut tensor_map:HashMap<String, Tensor::<B, 1>> = HashMap::new();
    let mut label_map:HashMap<String, i64> = HashMap::new();

    let size = db_embeddings.len();
    info!("size of the tensor map: {}", size);

    for (index,embedding) in db_embeddings.iter().enumerate() {
        let label = embedding.token;
        let one_hot = Tensor::<B, 1>::one_hot(index, size);

        tensor_map.insert(label.to_string(), one_hot);
        label_map.insert(label.to_string(), index as i64);
    }

    (tensor_map, label_map)
}