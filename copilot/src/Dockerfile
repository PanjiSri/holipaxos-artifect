FROM golang:1.22-alpine

WORKDIR /home/copilot/src
COPY . .

# build the server-side program
RUN go build -o bin/master master/master.go
RUN go build -o bin/server server/server.go

CMD ["./bin/server"]