use std::collections::HashMap;
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

impl DataNode {
    pub fn generate_nodes(data:&Vec<f64>, range:f64) -> Vec<DataNode> {
        let mut nodes:Vec<DataNode> = Vec::new();
        //sort data
        let mut sorted_data = data.clone();
        sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let node = DataNode::new(sorted_data[0]);
        nodes.push(node);

        let mut selected_node = &mut nodes[0];

        //loop through sorted data
        for i in 1..sorted_data.len(){
            let selected_data = sorted_data[i];
            let diff = selected_data.abs() - selected_node.members[0].abs();
            let perc = diff.abs() / selected_node.members[0].abs();
            if perc <= range{
                selected_node.add_member(selected_data);
            }else{
                let mut node = DataNode::new(selected_data);
                node.members.push(selected_data);
                nodes.push(node);
                let last_index = nodes.len()-1;
                selected_node = &mut nodes[last_index];
            }
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
            nodes[i].initialize_edges(&averages);
        }
    }
    fn initialize_edges(&mut self, averages:&Vec<f64>){
        for avg in averages{
            let edge  = Edge{
                value:*avg,
                score:0.0,
                weight:0.0,
            };
            self.edges.push(edge);
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
        for node in nodes{
            for (i, data) in averaged_data.iter().enumerate(){
                if  &node.average == data{
                    let data_copy = averaged_data.clone();
                    let (left,right) = data_copy.split_at(i);

                    for (index,entry) in left.iter().enumerate(){
                        let edge = node.edges.iter_mut().find(|x| x.value == *entry).unwrap();
                        edge.score += index as f64;
                    }
                    for (index,entry) in right.iter().rev().enumerate(){
                        let edge = node.edges.iter_mut().find(|x| x.value == *entry).unwrap();
                        edge.score += index as f64;
                    }
                }
            }
        }
    }

    pub fn set_weights(nodes: &mut Vec<DataNode>){
        for node in nodes{
            //find total score
            let total_score:f64 = node.edges.iter().map(|x| x.score).sum();
            for edge in &mut node.edges{
                let weight = edge.score /total_score ;
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


