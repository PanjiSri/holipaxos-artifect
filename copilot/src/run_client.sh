NUMBER=$1

for i in $(seq 1 "$NUMBER"); do
   ./bin/clientmain -id=${i} -twoLeaders=true -e=false -c=0 -check=true -w=100 -p=8 -runtime=300 -tput_interval_in_sec=5 -q=30000 &
   # ./bin/client -e=true -c=0 -w=50 -p=8 -q=300000 &
done

wait

