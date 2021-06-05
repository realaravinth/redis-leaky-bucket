#!/bin/sh
readonly NUM_REQUESTS=1000000

echo "running set and get without pipelining" 
redis-benchmark -n 1000000 -t set,get -q

echo "leaky bucket without piplining"
redis-benchmark -n $NUM_REQUESTS -q LBUCKET.COUNT mycounter 45

echo "running set and get with pipelining" 
redis-benchmark -n 1000000 -t set,get -q -P 16

echo "leaky bucket with piplining"
redis-benchmark -P 16 -n $NUM_REQUESTS -q LBUCKET.COUNT mycounter 45
