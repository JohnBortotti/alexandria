#!/bin/bash

# todo:
# provide concurrent read 
# provide concurrent write? and how?
# test how much bloom filter increase read time

DB_URL="192.30.103:5000" 
COLLECTION_NAME="test"
NUM_REQUESTS=100

generate_random_string() {
  LENGTH=$1
  tr -dc A-Za-z0-9 </dev/urandom | head -c $LENGTH
}

write_test() {
  local total_time=0
  local success_count=0

  for i in $(seq 1 $NUM_REQUESTS); do
    KEY=$(generate_random_string 8)
    VALUE=$(generate_random_string 16)
    START=$(date +%s%3N)
    RESPONSE=$(curl -L -s -o /dev/null -w "%{http_code}" -X POST -d "write $COLLECTION_NAME $KEY $VALUE" $DB_URL)
    END=$(date +%s%3N)
    DURATION=$((END-START))
    total_time=$((total_time + DURATION))

    if [ "$RESPONSE" -eq 200 ]; then
      success_count=$((success_count + 1))
    fi
  done

  echo "Total write time: $total_time ms"
  echo "Total successful writes: $success_count"
  echo "Average write time: $((total_time / NUM_REQUESTS)) ms"
}

read_test() {
  local total_time=0
  local success_count=0

  for i in $(seq 1 $NUM_REQUESTS); do
    KEY=$(generate_random_string 8)
    START=$(date +%s%3N)
    RESPONSE=$(curl -L -s -o /dev/null -w "%{http_code}" -X POST -d "get $COLLECTION_NAME $KEY" $DB_URL)
    END=$(date +%s%3N)
    DURATION=$((END-START))
    total_time=$((total_time + DURATION))

    if [ "$RESPONSE" -eq 200 ]; then
      success_count=$((success_count + 1))
    fi
  done

  echo "Total read time: $total_time ms"
  echo "Total successful reads: $success_count"
  echo "Average read time: $((total_time / NUM_REQUESTS)) ms"
}

benchmark() {
  echo "Starting write test..."
  write_test
  echo "Write test completed."
  echo ""
  echo "Starting read test..."
  read_test
  echo "Read test completed."
}

benchmark
