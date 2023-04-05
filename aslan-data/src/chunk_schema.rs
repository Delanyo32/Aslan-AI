use std::collections::HashMap;

#[derive(Debug)]
pub struct AslanDataChunks{
    flat_data: Vec<f64>,
}

#[derive(Debug)]
pub struct Node{
    key:String,
    pub data:f64,
    pub connected_nodes_before:Vec<f64>,
    pub connected_nodes_after:Vec<f64>,
}

#[derive(Debug)]
pub struct NodeSet{
    pub nodes:HashMap<String,Node>,
}

impl NodeSet{
    pub fn new() -> Self {
        NodeSet {
            nodes: HashMap::new(),
        }
    }
    pub fn add_node(&mut self, node:Node) -> &Self {
        self.nodes.insert(node.key.clone(), node);
        self
    }

    pub fn parse_data_chunks(&mut self, data_chunks:AslanDataChunks) -> &Self{
        for (i,data_chunk) in data_chunks.flat_data.iter().enumerate(){
            if self.nodes.contains_key(&data_chunk.to_string()){
                let node = self.nodes.get_mut(&data_chunk.to_string()).unwrap();
                //add second node to array of connected nodes
                if i+1 < data_chunks.flat_data.len() {
                    node.connected_nodes_after.push(data_chunks.flat_data[i+1]);
                }
                if i != 0 {
                    node.connected_nodes_before.push(data_chunks.flat_data[i-1]);
                }
            }else{ 
                let mut node = Node {
                    key:data_chunk.to_string(),
                    data: *data_chunk,
                    connected_nodes_before: Vec::new(),
                    connected_nodes_after: Vec::new(),
                };
                if i+1 < data_chunks.flat_data.len() {
                    node.connected_nodes_after.push(data_chunks.flat_data[i+1]);
                }
                if i != 0 {
                    node.connected_nodes_before.push(data_chunks.flat_data[i-1]);
                }
                self.add_node(node);
            }
        }

        self
    }

    pub fn generate_nodes(&mut self, flat_data:&Vec<f64>) -> Self{
        let mut nodeset = NodeSet::new();
        for (i,data_chunk) in flat_data.iter().enumerate(){
            let search_key = *data_chunk;
            let search_results = NodeSet::fuzzy_search(search_key, &self);
            for search_result in search_results{
                    let node = nodeset.nodes.get_mut(&search_result).unwrap();
                    //add second node to array of connected nodes
                    if i+1 < flat_data.len() {
                        node.connected_nodes_after.push(flat_data[i+1]);
                    }
                    if i != 0 {
                        node.connected_nodes_before.push(flat_data[i-1]);
                    }
            }
            
            if !(self.nodes.contains_key(&data_chunk.to_string())){ 
                let mut node = Node {
                    key:data_chunk.to_string(),
                    data: *data_chunk,
                    connected_nodes_before: Vec::new(),
                    connected_nodes_after: Vec::new(),
                };
                if i+1 < flat_data.len() {
                    node.connected_nodes_after.push(flat_data[i+1]);
                }
                if i != 0 {
                    node.connected_nodes_before.push(flat_data[i-1]);
                }
                nodeset.add_node(node);
            }
        }
        
        nodeset
    }

    //fuzzy search gives a range of values close to the search value
    fn fuzzy_search(search:f64,data:&NodeSet)->Vec<String>{
        let mut search_results = Vec::new();
        if data.nodes.contains_key(&search.to_string()){
            search_results.push(search.to_string());
        }
        let keys:Vec<&String> = data.nodes.keys().collect();
        let mut keys_f64:Vec<f64> = keys.iter().map(|x| x.parse::<f64>().unwrap()).collect();
        keys_f64.push(search);
        keys_f64.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let index = keys_f64.iter().position(|x| *x == search).unwrap();
        if index != 0{
            search_results.push(keys_f64[index-1].to_string());
        }
        if index != keys_f64.len()-1{
            search_results.push(keys_f64[index+1].to_string());
        }
        search_results
    }

}
impl Node{
    pub fn new(data:f64)->Self{
        Node{
            key:data.to_string(),
            data,
            connected_nodes_before:Vec::new(),
            connected_nodes_after:Vec::new(),
        }
    }
}

impl AslanDataChunks{
    pub fn new(data :Vec<f64>)->AslanDataChunks{
        AslanDataChunks{
            flat_data: data,
        }
    }

    //rename this normalize
    pub fn parse_linear_data(self)->Self{
        let result = self.flat_data.iter()
        .enumerate()
        .map(|(i,x)|{
            if i<self.flat_data.len()-1 {
                let next_x = self.flat_data[i+1];
                let diff = next_x - x;
                let diff = (diff * 100.0).round() / 100.0;
                diff
            }else{
                0.0
            }
        }).collect::<Vec<f64>>();
        AslanDataChunks{
            flat_data: result,
        }
    }
    pub fn normalize_data(data:&Vec<f64>)->Vec<f64>{
        let result = data.iter()
        .enumerate()
        .map(|(i,x)|{
            if i<data.len()-1 {
                let next_x = data[i+1];
                let diff = next_x - x;
                let diff = (diff * 100.0).round() / 100.0;
                diff
            }else{
                0.0
            }
        }).collect::<Vec<f64>>();
        result
    }
}