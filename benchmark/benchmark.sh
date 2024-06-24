#!/bin/bash

DB_URL="192.30.101:5000" 
COLLECTION_NAME="test"
NUM_REQUESTS=100
CONCURRENT_REQUESTS=1

generate_random_string() {
  LENGTH=$1
  tr -dc A-Za-z0-9 </dev/urandom | head -c $LENGTH
}

new_collection() {
	RESPONSE=$(curl -L -s -o /dev/null -w "%{http_code}" -X POST -d "create $COLLECTION_NAME" $DB_URL)
}

write_test() {
  local total_time=0
  local success_count=0

  for i in $(seq 1 $NUM_REQUESTS); do
    KEY=$(generate_random_string 8)
    VALUE=$(generate_random_string 16)
    START=$(date +%s%N)
    RESPONSE=$(curl -L -s -o /dev/null -w "%{http_code}" -X POST -d "write $COLLECTION_NAME $KEY $VALUE" $DB_URL)
    END=$(date +%s%N)
    DURATION=$((END-START))
    total_time=$((total_time + DURATION))

    if [ "$RESPONSE" -eq 200 ]; then
      success_count=$((success_count + 1))
    fi
  done

  total_time_sec=$(echo "scale=2; $total_time / 1000000000" | bc)
  average_time_sec=$(echo "scale=2; $total_time_sec / $NUM_REQUESTS" | bc)

  echo "Total write time: $total_time_sec seconds"
  echo "Total successful writes: $success_count"
  echo "Average write time: $average_time_sec seconds"
}

read_test() {
  local total_time=0
  local success_count=0

  for i in $(seq 1 $NUM_REQUESTS); do
    KEY=$(generate_random_string 8)
    START=$(date +%s%N)
    RESPONSE=$(curl -L -s -o /dev/null -w "%{http_code}" -X POST -d "get $COLLECTION_NAME $KEY" $DB_URL)
    END=$(date +%s%N)
    DURATION=$((END-START))
    total_time=$((total_time + DURATION))

    if [ "$RESPONSE" -eq 200 ]; then
      success_count=$((success_count + 1))
    fi
  done

  total_time_sec=$(echo "scale=2; $total_time / 1000000000" | bc)
  average_time_sec=$(echo "scale=2; $total_time_sec / $NUM_REQUESTS" | bc)

  echo "Total read time: $total_time_sec seconds"
  echo "Total successful reads: $success_count"
  echo "Average read time: $average_time_sec seconds"
}

read_test_concurrent() {
  local total_time=0
  local success_count=0
  local pids=()

  tmpfile=$(mktemp)

  for i in $(seq 1 $NUM_REQUESTS); do
    KEY=$(generate_random_string 8)
    (
      START=$(date +%s%N)
      RESPONSE=$(curl -L -s -o /dev/null -w "%{http_code}" -X POST -d "get $COLLECTION_NAME $KEY" $DB_URL)
      END=$(date +%s%N)
      DURATION=$((END-START))

      if [ "$RESPONSE" -eq 200 ]; then
        echo "$DURATION 1" >> "$tmpfile"
      else
        echo "$DURATION 0" >> "$tmpfile"
      fi
    ) &
    pids+=($!)

    if (( i % CONCURRENT_REQUESTS == 0 )); then
      wait "${pids[@]}"
      pids=()
    fi
  done

  wait "${pids[@]}"

  while IFS=' ' read -r duration success; do
    total_time=$((total_time + duration))
    success_count=$((success_count + success))
  done < "$tmpfile"

  rm "$tmpfile"

  total_time_sec=$(echo "scale=2; $total_time / 1000000000" | bc)
  average_time_sec=$(echo "scale=2; $total_time_sec / $NUM_REQUESTS" | bc)

  echo "Total read time: $total_time_sec seconds"
  echo "Total successful reads: $success_count"
  echo "Average read time: $average_time_sec seconds"
}

benchmark() {
  echo "Creating collection $COLLECTION_NAME..."
  new_collection
  echo ""
  echo "Starting write test..."
  write_test
  echo "Write test completed."
  echo ""
  echo "Starting read test..."
  read_test
  echo""
  echo "Starting concurrent read test..."
  read_test_concurrent
  echo "Read test completed."
}

benchmark
