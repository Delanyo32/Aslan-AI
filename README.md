# Aslan AI
Convert various libraries worker containers(http services)

# Steps
- Use data to capture and transform data
- Use Wavereduce to generate possibilities
- Use bootstraping to find the most likely options
- gleam infromation about the most likely option
- Evaluate decision accuracy
- Use traveling salesman algorithm to improve nodes

## Feature Components
- Data Collection and Parsing
- Wave reduce (to generate possible outcomes for testing)
- Bootstraping (to narrow down the possibilites)

## Testing and Improving
- Test and find best solutions created by wave reduce (can yeild very small standard deviation)
- Test to see if multiple iterations of bootstraping improves the standard deviation 
- Test to see if bootstraping with the best standard div generated from wavereduce improves result
- Test which standard divs affects the trend direction the most
- Test the simulated profits
- Test moving wavereduce and boostraping into training algorithm, where we use the iterations to improve the node neighbors
- Test various timing structures for the data. Convert to using "partitions" as a terminology
- Diagram out flow of data and functions to help build API
- Build stucture to test different implementations of the code
- Build API for app to streamline workflow

https://zliu.org/post/weighted-random/

## TODO
- Use Sureal DB for node modeling and data storage
- Use nearest neighbor to improve selection directions
- Add nearest node for node connection
- implement probabilistic selection
- increase amount of bootstraping iterations with test
- improving standard deviation 
- decouple from actual date time for bar data
- Generate Test Data focus on connecting actual to test data
- redesign engine structure to make it more generic
- refactor data to make it more generic
- switch to worker architecture
- evaluate correctness
- improve accuracy
- propergation across clusters of posibilities to reduce relm of posibilities (remove solutions and weight which do not match current data)
- run bootstrapping to generate posibilities to improve weights
- decouple code
- increase scope of data
- load and unload node data
- metrics and logging

## Done
- Split data 
- Add bidirectional generation
- weight segment connections
- Gather test data
- create a chunk
- convert values to relative values or distance maagnitudes
- Remove duplicate nodes
- initialize connection weights

## Configuration Variables
- Stock symbol
- Date range
- Normalization value
- TimeFrame


## Bootstraping For weighing
- create a pool of node ids
- randomly select a node
- randomly select other nodes that follow it
- use standard deviation to check if it is valid 
- update the weights based on the iteration
- run multiple iterations 

# Ant Colony Optimization
- Create a matrix of the vector data with weights
- Run a bootstraping round
- Log the standard deviation 
- Add the the inverse of the standard deviation to the weights of the selected values during the bootstaping phase
- retry bootstraping until the smallest standard deviation has been achived or something akin to that
- Over time and a number of iterations, the solution should continualy get better based on the weights

## Architecture
Pipeline Architecture
- Data fetching 
- Parsing 
- Bootstraping


## Notes
- Load Keys from environmment file
- When mixing mutability and immutability always copy the immutable before mutating it. 
- Chunking: using similar and related data to group the data. recognize preliminary patterns
- Compare to random stock picking and trades
- Using the traveling sales man algorithms to inprove predictions and get closer to the solution


## Useful comands
``
docker pull delanyo32/task-scheduler:latest
docker pull delanyo32/aslan-core:latest
docker pull redis
docker pull datadog/agent

docker network create --driver bridge aslan-core-net

docker run -it -p 9000:9000 -d --net "aslan-core-net" delanyo32/aslan-core bash 

docker container exec -it 2e60b79af999 bash 
curl -w '\n' http://chart-redis-c816035e:6379/ping
curl -w '\n' http://chart-taskschedulerdeploymentservice-c80e97cf:8080/health
curl -w '\n' http://chart-aslancoreservice-c8f2509a:9000

curl -Is http://chart-redis-c816035e:6379 | head -1


docker run --name redis -p 6379:6379 -d --net "aslan-core-net" redis 
docker run -dp 9000:9000 delanyo32/aslan-core --network aslan-core-net
docker run -dp 8080:8080 delanyo32/task-scheduler
docker run -d --cgroupns host --pid host --name dd-agent -v /var/run/docker.sock:/var/run/docker.sock:ro -v /proc/:/host/proc/:ro -v /sys/fs/cgroup/:/host/sys/fs/cgroup:ro -e DD_API_KEY=<DATADOG_API_KEY> gcr.io/datadoghq/agent:7

helm install aslan-logging -f values.yaml  --set datadog.apiKey=f220fb795c794aac53c8d9c885c42526 datadog/datadog --set targetSystem=linux

kubectl create deployment --image redis redis 
kubectl expose deployment my-nginx --port=80 --type=LoadBalancer

npm run compile && cdk8s synth 
``