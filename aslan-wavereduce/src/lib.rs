use aslan_data::{self, Node,NodeSet, DataNode, Edge};
use rand::prelude::*;
//seed: some inital data for the graph
//entropy: possible states of the each cell
//cell: contains a state
//result: a generated solution

#[derive(Debug)]
pub struct WaveReduce{
    seed:f64,
    number_of_cells:usize,
    number_of_results:usize,
}
#[derive(Debug)]
pub struct WaveReduceCell{
    pub state:f64,
}
#[derive(Debug)]
pub struct WaveReduceResult{
    pub result:Vec<WaveReduceCell>,
}
#[derive(Debug)]
pub struct WaveReduceSolution{
    pub results:Vec<WaveReduceResult>,
}

#[derive(Debug)]
pub struct WaveReduceSummary{
    partition_total:f64,
    solution_total:f64,
    difference:f64,
    solution_index:usize,
}

impl WaveReduce {
    pub fn new(seed:f64, number_of_cells:usize, number_of_results:usize)->Self{
        WaveReduce{
            seed,
            number_of_cells,
            number_of_results,
        }
    }
    
    fn select_first_node_index(seed:f64,data:&Vec<DataNode>)->usize{
        //find node index with average equal to seed
        let index = WaveReduce::fuzzy_search(seed, data);
        index
    }

    //fuzzy search gives a range of values close to the search value
    fn fuzzy_search(search:f64,data:&Vec<DataNode>)->usize{
        let mut index = 0;
        let mut previous_diff = 0.0;
        for i in 0..data.len(){
            let difference = (data[i].average - search).abs();
            if difference < previous_diff{
                index = i;
                previous_diff = difference;
            }
        }
        index
    }

    // add function to fuzzy search for the closest match for the seed
    pub fn generate_results(self ,data:&Vec<DataNode>)->WaveReduceSolution{
        let mut solution = WaveReduceSolution{
            results:Vec::new(),
        };
        let mut selected_node_index  = WaveReduce::select_first_node_index(self.seed,data);
        let mut selected_node = &data[selected_node_index];
        //for loop to generate results
        for _ in 0..self.number_of_results{
            let mut wave_result = WaveReduceResult{
                result:Vec::new(),
            };
            for _ in 0..self.number_of_cells{
                //randomly select a node from the selected node's neighbors
                let edge_index = WaveReduce::get_weighted_random(&selected_node.edges);

                let cell = WaveReduceCell{
                    state:selected_node.edges[edge_index].value,
                };

                let new_seed = selected_node.edges[edge_index].value;
                selected_node_index  = WaveReduce::select_first_node_index(new_seed,data);
                selected_node = &data[selected_node_index];

                wave_result.result.push(cell);
            }

            solution.results.push(wave_result);
        }
        

        solution
    }

    fn get_weighted_random(edges: &Vec<Edge>)->usize{
        let mut rng = rand::thread_rng();
        let mut total_weight:f64 = 0.0;
        for edge in edges{
            total_weight += edge.weight;
        }
        let mut random_number:f64 = rng.gen_range(0.0..total_weight);
        let mut index = 0;
        for edge in edges{
            random_number -= edge.weight;
            if random_number <= 0.0{
                break;
            }
            index += 1;
        }
        index
    }
    
}

impl WaveReduceSolution {

    pub fn flatten_results(data:Vec<&WaveReduceResult>)->Vec<Vec<f64>>{
        let mut flattened_results = Vec::new();
        for result in data{
            let mut flattened_result = result.result.iter().map(|x| x.state).collect();
            flattened_results.push(flattened_result);
        }
        flattened_results
    }

    pub fn get_result_summary(&self, partition:&Vec<f64>)->Vec<WaveReduceSummary>{
        let mut summary = Vec::new();
        //add up data in partitions
        let sum = partition.iter().fold(0.0, |sum, x| sum + x);

        for (index,result) in self.results.iter().enumerate(){
            let mut flattened_result:Vec<f64> = result.result.iter().map(|x| x.state).collect();
            // sum flattened result
            let result_sum = flattened_result.iter().fold(0.0, |sum, x| sum + x);
            //calculate difference
            let difference = (sum - result_sum).abs();
            let summary_item = WaveReduceSummary{
                partition_total:sum,
                solution_total:result_sum,
                difference,
                solution_index:index,
            };

            summary.push(summary_item);
        }
        summary
    }

    pub fn get_top_results(&self, summary:Vec<WaveReduceSummary>, number_of_results:usize)->Vec<&WaveReduceResult>{
        let mut top_results = Vec::new();
        let mut sorted_summary = summary;
        sorted_summary.sort_by(|a, b| a.difference.partial_cmp(&b.difference).unwrap());
        for i in 0..number_of_results{
            let index = sorted_summary[i].solution_index;
            top_results.push(&self.results[index]);
        }
        top_results
    }

    pub fn get_random_results(&self, number_of_results:usize)->Vec<&WaveReduceResult>{
        let mut random_results = Vec::new();
        let mut rng = rand::thread_rng();
        for _ in 0..number_of_results{
            let index = rng.gen_range(0..self.results.len());
            random_results.push(&self.results[index]);
        }
        random_results
    }
        
}