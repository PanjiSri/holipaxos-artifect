serverPort=$1

bin/server -port=${serverPort} -e=false -p=8 -copilot=true -exec=true -dreply=true
