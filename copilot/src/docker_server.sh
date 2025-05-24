serverPort=$3
master_addr=$1
my_addr=$2

docker run -id -d --rm -p ${serverPort}:${serverPort} --name server${serverPort} --net host copilot bin/server -maddr=${master_addr} -addr=${my_addr}  -port=${serverPort} -e=false -p=8 -copilot=true -exec=true -dreply=true
