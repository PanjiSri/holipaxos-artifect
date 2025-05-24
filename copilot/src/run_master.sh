serverPort=$1

pids=()

cleanup() {
    for pid in "${pids[@]}"; do
        kill "$pid"
    done
    rm *.txt
}

trap cleanup SIGINT

bin/master -N=2 -twoLeaders=true &
pids+=($!)

bin/server -maddr=172.31.22.119 -addr=172.31.22.119 -port=${serverPort} -e=false -p=8 -copilot=true -exec=true -dreply=true &

pids+=($!)

wait
