FROM golang:alpine3.19

WORKDIR /home/go-grpc
COPY . .

# build the server-side program
RUN go build -o bin/replicant main.go

CMD ["./bin/replicant"]
