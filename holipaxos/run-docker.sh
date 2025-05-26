node_id=$1
serverPort=$2

docker run -it -d --rm -p ${serverPort}:${serverPort} --name server${serverPort} --net host replicant bin/replicant -id=${node_id}
