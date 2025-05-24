NUMBER=$2
REQ_NUMBER=$1

mkdir -p tput-latency-data

pids=()

cleanup() {
    for pid in "${pids[@]}"; do
        kill "$pid"
    done
    rm *.txt
}

trap cleanup SIGINT

for i in $(seq 1 "$NUMBER"); do
    ./bin/client -id=${i} \
	         -maddr=172.31.23.214 \
		 -tput_interval_in_sec=5 -c=0 -check=true -w=100 -p=8 -runtime=120 -q=${REQ_NUMBER} |tee tput-latency-data/${NUMBER}-${i}.txt &
    pids+=($!)
done

wait

