use std::collections::HashMap;
use polars::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Debug,Serialize, Deserialize,Clone)]
pub struct DataNode {
    pub average:f64,
    pub members : Vec<f64>,
    pub edges : Vec<Edge>
}

#[derive(Debug,Serialize, Deserialize,Clone)]
pub struct Edge {
    pub value: f64,
    pub score: f64,
    pub weight: f64,
}

impl Edge {
    pub fn new(value:f64) -> Self {
        Edge {
            value,
            score : 0.0,
            weight : 0.0,
        }
    }
}

impl DataNode {
 
    pub fn generate_nodes(data:&Vec<f64>, _range:f64) -> Vec<DataNode> {
        let mut nodes:Vec<DataNode> = Vec::new();

        //sort data
        let sorted_data = data.clone();
        let sorted_series = Series::new("sorted", sorted_data);
        let sort = sorted_series.sort(false);
        let unique = sort.unique().unwrap();

        let result = match unique.f64(){
            Ok(values) => values.into_no_null_iter().collect(),
            Err(_) => Vec::new(),
        };
        // create an array of nodes from the unique values
        let lenght = result.len();
        let edges_array = result.clone();

        for value in result{
            let mut node = DataNode::new(value);
            let edges:Vec<Edge> = edges_array.iter().map(|x| Edge::new(*x)).collect();
            node.edges = edges;
            nodes.push(node);
        }
       
        nodes
    }

    pub fn new(data:f64)->Self{
        DataNode{
            average:data,
            members:vec![data],
            edges:Vec::new()
        }
    }

    pub fn add_member(&mut self, data:f64){
        self.members.push(data);
        let sum:f64 = self.members.iter().sum();
        let average = sum / self.members.len() as f64;
        let average = (average * 100.0).round() / 100.0;
        self.average = average
    }

    pub fn initialize_node_edges(nodes: &mut Vec<DataNode>){
        let averages:Vec<f64> = nodes.iter().map(|x| x.average).collect();
        for i in 0..nodes.len(){
            let edges:Vec<Edge> = vec![Edge::new(averages[i]); averages.len()];
            nodes[i].edges = edges;
        }
    }
    

    //converts the data array to the averages of the data
    pub fn parse_data(nodes:&Vec<DataNode>, data: &Vec<f64>)->Vec<f64>{
        //make copy of the data
        let mut result = data.clone();
        for node in nodes{
            for member in &node.members{
                //find all occurrences of member in data
                let mut indices:Vec<usize> = Vec::new();
                for (i, x) in result.iter().enumerate(){
                    if x == member{
                        indices.push(i);
                    }
                }
                //replace all occurrences of member in data with node average
                for i in indices{
                    result[i] = node.average;
                }
            }
        }
        result
    }

    pub fn set_distance_scores (nodes: &mut Vec<DataNode>, averaged_data: &Vec<f64>){
        // generate an array of all indexes of the occurance of the data in the averaged data
        // load all edges into a hashmap
        // for each index split the at the index 
        // for each [left, right] update the score of the edge decending in value further from the index

        for node in nodes{
            // generate an array of all indexes of the occurance of the data in the averaged data
            let indexes:Vec<usize> = averaged_data.iter().enumerate().filter_map(|(i, x)| if x == &node.average { Some(i) } else { None }).collect();

            // load all edges into a hashmap where the key is the value and the value is its index in the edges array
            let edges:HashMap<String, usize> = node.edges.iter().enumerate().map(|(i, item)| (item.value.to_string(), i)).collect();

            // for each index split the at the index
            for index in indexes{
                let data_copy = averaged_data.clone();
                let (left,right) = data_copy.split_at(index);

                // for each [left, right] update the score of the edge decending in value further from the index
                for (index,entry) in left.iter().enumerate(){
                    // find edge index in array 
                    let edge_index = edges.get(&entry.to_string()).unwrap().clone();
                    // update score in the node edges array
                    let edge = node.edges.get_mut(edge_index).unwrap();
                    edge.score += index as f64;
                }

                for (index,entry) in right.iter().rev().enumerate(){
                    // find edge index in array 
                    let edge_index = edges.get(&entry.to_string()).unwrap().clone();
                    // update score in the node edges array
                    let edge = node.edges.get_mut(edge_index).unwrap();
                    edge.score += index as f64;
                }
            }
        }
    }

    pub fn set_weights(nodes: &mut Vec<DataNode>) {
        for node in nodes {
            let total_score: f64 = node.edges.iter().map(|x| x.score).sum();
            let edges_len = node.edges.len();
    
            for i in 0..edges_len {
                let edge = &mut node.edges[i];
                let weight = edge.score / total_score;
                edge.weight = weight;
            }
        }
    }

    pub fn init_map(nodes: &Vec<DataNode>, data: &Vec<f64>)->(HashMap<usize,f64>,HashMap<String,f64>){
        // dictionary mapping the data to the index of the data
        let mut data_dict:HashMap<usize,f64> = HashMap::new();
        for i in 0..data.len(){
            data_dict.insert(i,data[i]);
        }

        //dictionary mapping the node to the average of the node
        let mut node_dict:HashMap<String,f64> = HashMap::new();

        for node in nodes{
            let average = node.average;
            for member in node.members.iter(){
                let member_key = member.to_string();
                node_dict.insert(member_key, average);
            }
        }
        (data_dict,node_dict)
    }

    pub fn update_edge(self: &mut DataNode, edge:f64, update_score:f64){
        self.edges.iter_mut().find(|x| x.value == edge).unwrap().score += update_score;
    }

}


